use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use thiserror::Error;

use crate::{
    AuthorizationError, AuthorizationPort, BackendOperation, BackendOperationRequest, BackendState,
    Clock, ConnectivityPort, CreateTranscriptionRequest, DomainError, Failure, FailureCategory,
    FinalTranscript, OperationEvent, OperationEventSink, OperationPhase, OperationRepository,
    RepositoryError, SourceAudioError, SourceAudioId, SourceAudioPort, SubmissionFingerprint,
    TranscriptionOperation, TranscriptionOperationId, TranscriptionOptions, TranscriptionPort,
    TranscriptionPortError, UploadObservation, UploadProgressSink,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationOutcome {
    pub operation: TranscriptionOperation,
    pub transcript: Option<FinalTranscript>,
}

impl OperationOutcome {
    fn operation(operation: TranscriptionOperation) -> Self {
        Self {
            operation,
            transcript: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Source(#[from] SourceAudioError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("operation identity was reused with changed source or options")]
    IdempotencyMismatch,
    #[error("operation has a conflicting terminal outcome")]
    TerminalConflict,
}

pub struct TranscriptionService {
    backend: Arc<dyn TranscriptionPort>,
    sources: Arc<dyn SourceAudioPort>,
    operations: Arc<dyn OperationRepository>,
    authorization: Arc<dyn AuthorizationPort>,
    connectivity: Arc<dyn ConnectivityPort>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn OperationEventSink>,
}

impl TranscriptionService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        backend: Arc<dyn TranscriptionPort>,
        sources: Arc<dyn SourceAudioPort>,
        operations: Arc<dyn OperationRepository>,
        authorization: Arc<dyn AuthorizationPort>,
        connectivity: Arc<dyn ConnectivityPort>,
        clock: Arc<dyn Clock>,
        events: Arc<dyn OperationEventSink>,
    ) -> Self {
        Self {
            backend,
            sources,
            operations,
            authorization,
            connectivity,
            clock,
            events,
        }
    }

    pub async fn submit(
        &self,
        source_id: SourceAudioId,
        options: TranscriptionOptions,
    ) -> Result<OperationOutcome, ApplicationError> {
        let source = self.sources.inspect(&source_id).await?;
        if source.id != source_id {
            return Err(ApplicationError::Source(SourceAudioError::Invalid));
        }
        let fingerprint = SubmissionFingerprint::derive(&source, &options);
        let candidate = TranscriptionOperation::new(
            TranscriptionOperationId::new(),
            source_id,
            fingerprint.clone(),
            options.clone(),
        );
        let existing = self.operations.get_or_create(candidate).await?.operation;
        if existing.fingerprint() != &fingerprint || existing.options() != &options {
            return Err(ApplicationError::IdempotencyMismatch);
        }

        match existing.phase() {
            OperationPhase::Completed | OperationPhase::Queued | OperationPhase::Processing => {
                self.status(existing.id().clone()).await
            }
            OperationPhase::Cancelled | OperationPhase::TerminalFailure => {
                Ok(OperationOutcome::operation(existing))
            }
            OperationPhase::Uploading
            | OperationPhase::Cancelling
            | OperationPhase::CleanupPending => Ok(OperationOutcome::operation(existing)),
            OperationPhase::Uncertain if existing.backend_operation_id().is_some() => {
                self.status(existing.id().clone()).await
            }
            _ => self.dispatch_create(existing, source).await,
        }
    }

    pub async fn status(
        &self,
        operation_id: TranscriptionOperationId,
    ) -> Result<OperationOutcome, ApplicationError> {
        let current = self.operations.load(&operation_id).await?;
        if matches!(
            current.phase(),
            OperationPhase::Cancelled | OperationPhase::TerminalFailure
        ) {
            return Ok(OperationOutcome::operation(current));
        }
        let Some(backend_id) = current.backend_operation_id().cloned() else {
            return Ok(OperationOutcome::operation(current));
        };
        let authorization = match self.authorization.acquire().await {
            Ok(authorization) => authorization,
            Err(error) => return self.wait_for_authorization(current, error).await,
        };
        match self
            .backend
            .get(BackendOperationRequest {
                operation_id,
                backend_operation_id: backend_id,
                source_audio_id: current.source_audio_id().clone(),
                authorization,
            })
            .await
        {
            Ok(remote) => self.apply_backend(current, remote).await,
            Err(error) => self.apply_port_failure(current, error).await,
        }
    }

    pub async fn retry(
        &self,
        operation_id: TranscriptionOperationId,
    ) -> Result<OperationOutcome, ApplicationError> {
        let mut current = self.operations.load(&operation_id).await?;
        match current.phase() {
            OperationPhase::TerminalFailure
                if current.cleanup().needs_retry() && current.backend_operation_id().is_some() =>
            {
                if !current.terminal_cleanup_retry_ready(self.clock.now_ms()) {
                    return Ok(OperationOutcome::operation(current));
                }
                return self.dispatch_terminal_cleanup(current).await;
            }
            OperationPhase::Completed
            | OperationPhase::Cancelled
            | OperationPhase::TerminalFailure => {
                return Ok(OperationOutcome::operation(current));
            }
            OperationPhase::Queued | OperationPhase::Processing => {
                return self.status(operation_id).await;
            }
            OperationPhase::Uncertain if current.backend_operation_id().is_some() => {
                return self.status(operation_id).await;
            }
            OperationPhase::WaitingForAuthorization if current.backend_operation_id().is_some() => {
                return self.status(operation_id).await;
            }
            OperationPhase::Cancelling | OperationPhase::CleanupPending
                if current.backend_operation_id().is_some() =>
            {
                return self.dispatch_delete(current).await;
            }
            OperationPhase::Uploading => {
                debug_assert!(current.recover_interrupted_upload());
                current = self.commit(current).await?;
            }
            OperationPhase::Cancelling if current.backend_operation_id().is_none() => {}
            _ => {}
        }
        let source = self.sources.inspect(current.source_audio_id()).await?;
        let fingerprint = SubmissionFingerprint::derive(&source, current.options());
        if &fingerprint != current.fingerprint() {
            return Err(ApplicationError::IdempotencyMismatch);
        }
        self.dispatch_create(current, source).await
    }

    pub async fn cancel(
        &self,
        operation_id: TranscriptionOperationId,
    ) -> Result<OperationOutcome, ApplicationError> {
        let current = self.operations.load(&operation_id).await?;
        match current.phase() {
            OperationPhase::Cancelled => return Ok(OperationOutcome::operation(current)),
            OperationPhase::Completed | OperationPhase::TerminalFailure => {
                return Err(ApplicationError::TerminalConflict);
            }
            OperationPhase::Cancelling | OperationPhase::CleanupPending
                if current.backend_operation_id().is_some() =>
            {
                return self.dispatch_delete(current).await;
            }
            _ => {}
        }
        if current.backend_operation_id().is_none() && current.phase() == OperationPhase::Uploading
        {
            let mut cancelling = current;
            cancelling.begin_cancel()?;
            let cancelling = self.commit(cancelling).await?;
            self.backend.cancel_local(cancelling.id());
            return Ok(OperationOutcome::operation(cancelling));
        }
        if current.backend_operation_id().is_none() && current.phase() == OperationPhase::Cancelling
        {
            return Ok(OperationOutcome::operation(current));
        }
        if current.backend_operation_id().is_none() {
            let mut cancelled = current;
            cancelled.cancel_local()?;
            return Ok(OperationOutcome::operation(self.commit(cancelled).await?));
        }
        let mut cancelling = current;
        cancelling.begin_cancel()?;
        let cancelling = self.commit(cancelling).await?;
        if cancelling.terminal_winner().is_some() {
            return Err(ApplicationError::TerminalConflict);
        }
        self.dispatch_delete(cancelling).await
    }

    pub async fn recover(&self) -> Result<Vec<OperationOutcome>, ApplicationError> {
        let mut recovered = Vec::new();
        for mut operation in self.operations.list_unfinished().await? {
            if operation.recover_interrupted_upload() {
                operation = self.commit(operation).await?;
            }
            recovered.push(OperationOutcome::operation(operation));
        }
        recovered.sort_by(|left, right| {
            left.operation
                .id()
                .to_string()
                .cmp(&right.operation.id().to_string())
        });
        Ok(recovered)
    }

    async fn dispatch_create(
        &self,
        mut operation: TranscriptionOperation,
        source: crate::SourceDescriptor,
    ) -> Result<OperationOutcome, ApplicationError> {
        let reconciling_cancel =
            operation.cancel_requested() && operation.backend_operation_id().is_none();
        if !self.connectivity.is_online().await {
            if reconciling_cancel {
                return Ok(OperationOutcome::operation(operation));
            }
            if operation.phase() != OperationPhase::WaitingForNetwork {
                operation.mark_waiting_for_network()?;
                operation = self.commit(operation).await?;
            }
            return Ok(OperationOutcome::operation(operation));
        }
        operation.begin_upload(self.clock.now_ms())?;
        operation = self.commit(operation).await?;
        if operation.terminal_winner().is_some() {
            return Ok(OperationOutcome::operation(operation));
        }

        let authorization = match self.authorization.acquire().await {
            Ok(token) => token,
            Err(error) => return self.wait_for_authorization(operation, error).await,
        };
        let latest = self.operations.load(operation.id()).await?;
        if (latest.cancel_requested() || latest.phase() == OperationPhase::Cancelling)
            && !reconciling_cancel
        {
            return Ok(OperationOutcome::operation(latest));
        }
        operation = latest;
        if reconciling_cancel {
            self.backend.prepare_cancel_reconciliation(operation.id());
        }
        let (progress_sender, mut progress_receiver) = futures::channel::mpsc::unbounded();
        let request = CreateTranscriptionRequest {
            operation_id: operation.id().clone(),
            source,
            fingerprint: operation.fingerprint().clone(),
            options: operation.options().clone(),
            attempt: operation.attempt(),
            authorization,
            progress: Arc::new(ProgressRecorder(progress_sender)),
        };
        let create = self.backend.create(request).fuse();
        futures::pin_mut!(create);
        let result = loop {
            futures::select! {
                result = create => break result,
                observation = progress_receiver.next() => {
                    if let Some(observation) = observation {
                        persist_progress(&self.operations, &self.events, observation).await;
                    }
                }
            }
        };
        while let Ok(observation) = progress_receiver.try_recv() {
            persist_progress(&self.operations, &self.events, observation).await;
        }
        let latest = self.operations.load(operation.id()).await?;
        match result {
            Ok(remote) => self.apply_backend(latest, remote).await,
            Err(error) => self.apply_port_failure(latest, error).await,
        }
    }

    async fn dispatch_delete(
        &self,
        mut operation: TranscriptionOperation,
    ) -> Result<OperationOutcome, ApplicationError> {
        let backend_operation_id = operation
            .backend_operation_id()
            .cloned()
            .ok_or(DomainError::MissingBackendOperationId)?;
        let authorization = self.authorization.acquire().await?;
        match self
            .backend
            .delete(BackendOperationRequest {
                operation_id: operation.id().clone(),
                backend_operation_id,
                source_audio_id: operation.source_audio_id().clone(),
                authorization,
            })
            .await
        {
            Ok(remote) => self.apply_backend(operation, remote).await,
            Err(error) => {
                operation.mark_cleanup_uncertain(error.failure)?;
                let operation = self.commit(operation).await?;
                Ok(OperationOutcome::operation(operation))
            }
        }
    }

    async fn dispatch_terminal_cleanup(
        &self,
        mut operation: TranscriptionOperation,
    ) -> Result<OperationOutcome, ApplicationError> {
        let backend_operation_id = operation
            .backend_operation_id()
            .cloned()
            .ok_or(DomainError::MissingBackendOperationId)?;
        let authorization = match self.authorization.acquire().await {
            Ok(authorization) => authorization,
            Err(error) => {
                operation.record_terminal_cleanup_failure(
                    Failure::new(
                        "AUTHENTICATION_REQUIRED",
                        FailureCategory::UserActionable,
                        None,
                    )?,
                    self.clock.now_ms(),
                )?;
                let _ = self.commit(operation).await?;
                return Err(ApplicationError::Authorization(error));
            }
        };
        let remote = match self
            .backend
            .delete(BackendOperationRequest {
                operation_id: operation.id().clone(),
                backend_operation_id: backend_operation_id.clone(),
                source_audio_id: operation.source_audio_id().clone(),
                authorization,
            })
            .await
        {
            Ok(remote) => remote,
            Err(error) => {
                operation.record_terminal_cleanup_failure(error.failure, self.clock.now_ms())?;
                return Ok(OperationOutcome::operation(self.commit(operation).await?));
            }
        };
        if remote.id != backend_operation_id
            || remote.source_audio_id != *operation.source_audio_id()
        {
            return Err(DomainError::BackendIdentityMismatch.into());
        }
        operation.set_cleanup(remote.cleanup);
        Ok(OperationOutcome::operation(self.commit(operation).await?))
    }

    async fn wait_for_authorization(
        &self,
        mut operation: TranscriptionOperation,
        error: AuthorizationError,
    ) -> Result<OperationOutcome, ApplicationError> {
        if operation.terminal_winner().is_none() {
            operation.mark_waiting_for_authorization(Failure::new(
                "AUTHENTICATION_REQUIRED",
                FailureCategory::UserActionable,
                None,
            )?)?;
            let _ = self.commit(operation).await?;
        }
        Err(ApplicationError::Authorization(error))
    }

    async fn apply_port_failure(
        &self,
        mut operation: TranscriptionOperation,
        error: TranscriptionPortError,
    ) -> Result<OperationOutcome, ApplicationError> {
        if operation.terminal_winner().is_some() {
            return Ok(OperationOutcome::operation(operation));
        }
        if operation.cancel_requested() {
            operation.mark_cancel_reconciliation_needed(error.failure)?;
            return Ok(OperationOutcome::operation(self.commit(operation).await?));
        }
        if error.failure.is_authentication_required() {
            operation.mark_waiting_for_authorization(error.failure)?;
            return Ok(OperationOutcome::operation(self.commit(operation).await?));
        }
        operation.fail(error.failure, self.clock.now_ms())?;
        Ok(OperationOutcome::operation(self.commit(operation).await?))
    }

    async fn apply_backend(
        &self,
        mut operation: TranscriptionOperation,
        remote: BackendOperation,
    ) -> Result<OperationOutcome, ApplicationError> {
        let latest = self.operations.load(operation.id()).await?;
        if latest.revision() > operation.revision() {
            operation = latest;
        }
        if remote.source_audio_id != *operation.source_audio_id() {
            return self.malformed(operation).await;
        }
        if operation
            .backend_operation_id()
            .is_some_and(|known| known != &remote.id)
        {
            return if operation.terminal_winner().is_some() {
                Err(DomainError::BackendIdentityMismatch.into())
            } else {
                self.malformed(operation).await
            };
        }
        if operation.terminal_winner().is_some() {
            if operation.terminal_winner() == Some(crate::TerminalWinner::Completed)
                && remote.state == BackendState::Completed
            {
                let transcript = self.final_transcript_with_ids(&operation, &remote)?;
                return Ok(OperationOutcome {
                    operation,
                    transcript: Some(transcript),
                });
            }
            return Ok(OperationOutcome::operation(operation));
        }

        if operation.cancel_requested()
            && !matches!(
                remote.state,
                BackendState::Cancelled | BackendState::Deleted | BackendState::Deleting
            )
        {
            operation.observe_backend_active(
                remote.id,
                OperationPhase::Processing,
                remote.request_id,
                None,
            )?;
            operation.begin_cancel()?;
            let operation = self.commit(operation).await?;
            return Box::pin(self.dispatch_delete(operation)).await;
        }

        let transcript_candidate = match remote.state {
            BackendState::Queued => {
                let poll_at_ms = remote
                    .poll_after_ms
                    .map(|delay| self.clock.now_ms().saturating_add(delay));
                operation.observe_backend_active(
                    remote.id,
                    OperationPhase::Queued,
                    remote.request_id,
                    poll_at_ms,
                )?;
                operation.set_cleanup(remote.cleanup);
                None
            }
            BackendState::Processing => {
                let poll_at_ms = remote
                    .poll_after_ms
                    .map(|delay| self.clock.now_ms().saturating_add(delay));
                operation.observe_backend_active(
                    remote.id,
                    OperationPhase::Processing,
                    remote.request_id,
                    poll_at_ms,
                )?;
                operation.set_cleanup(remote.cleanup);
                None
            }
            BackendState::Completed => {
                let transcript = match self.final_transcript_with_ids(&operation, &remote) {
                    Ok(transcript) => transcript,
                    Err(_) => return self.malformed(operation).await,
                };
                operation.complete(Some(remote.id))?;
                operation.set_cleanup(remote.cleanup);
                Some(transcript)
            }
            BackendState::Failed => {
                let Some(failure) = remote.failure else {
                    return self.malformed(operation).await;
                };
                operation.observe_backend_active(
                    remote.id,
                    OperationPhase::Processing,
                    remote.request_id,
                    None,
                )?;
                operation.fail(failure, self.clock.now_ms())?;
                operation.set_cleanup(remote.cleanup);
                None
            }
            BackendState::Cancelled | BackendState::Deleted => {
                operation.confirm_cancel(remote.cleanup)?;
                None
            }
            BackendState::Deleting => {
                operation.confirm_cancel(remote.cleanup)?;
                None
            }
        };
        let operation = self.commit(operation).await?;
        let transcript = if operation.terminal_winner() == Some(crate::TerminalWinner::Completed) {
            transcript_candidate
        } else {
            None
        };
        Ok(OperationOutcome {
            operation,
            transcript,
        })
    }

    fn final_transcript_with_ids(
        &self,
        operation: &TranscriptionOperation,
        remote: &BackendOperation,
    ) -> Result<FinalTranscript, ApplicationError> {
        let result = remote.result.as_ref().ok_or(DomainError::BlankTranscript)?;
        Ok(FinalTranscript::new(
            operation.id().clone(),
            remote.id.clone(),
            result.text(),
            result.language.clone(),
        )?)
    }

    async fn malformed(
        &self,
        mut operation: TranscriptionOperation,
    ) -> Result<OperationOutcome, ApplicationError> {
        operation.fail_terminal(Failure::new(
            "MALFORMED_RESPONSE",
            FailureCategory::Terminal,
            None,
        )?)?;
        Ok(OperationOutcome::operation(self.commit(operation).await?))
    }

    async fn commit(
        &self,
        mut operation: TranscriptionOperation,
    ) -> Result<TranscriptionOperation, ApplicationError> {
        let expected_revision = operation.revision();
        let operation_id = operation.id().clone();
        operation.next_event_sequence();
        match self
            .operations
            .compare_and_swap(expected_revision, operation)
            .await
        {
            Ok(committed) => {
                self.events.emit(Self::event(&committed));
                Ok(committed)
            }
            Err(RepositoryError::RevisionConflict) => {
                let latest = self.operations.load(&operation_id).await?;
                if latest.terminal_winner().is_some() {
                    Ok(latest)
                } else {
                    Err(ApplicationError::Repository(
                        RepositoryError::RevisionConflict,
                    ))
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    fn event(operation: &TranscriptionOperation) -> OperationEvent {
        let progress_basis_points = operation.progress().map(|progress| {
            ((progress.supplied_bytes.saturating_mul(10_000) / progress.total_bytes).min(10_000))
                as u16
        });
        OperationEvent {
            operation_id: operation.id().clone(),
            sequence: operation.event_sequence(),
            attempt: operation.attempt(),
            phase: operation.phase(),
            progress_basis_points,
            failure_code: operation
                .cleanup_failure()
                .or_else(|| operation.failure())
                .map(|failure| failure.code.clone()),
            retry_at_ms: operation
                .retry()
                .map(|retry| retry.earliest_retry_at_ms)
                .or(operation.poll_at_ms()),
            cleanup: operation.cleanup().clone(),
        }
    }
}

struct ProgressRecorder(futures::channel::mpsc::UnboundedSender<UploadObservation>);

impl UploadProgressSink for ProgressRecorder {
    fn observe(&self, observation: UploadObservation) {
        let _ = self.0.unbounded_send(observation);
    }
}

async fn persist_progress(
    operations: &Arc<dyn OperationRepository>,
    events: &Arc<dyn OperationEventSink>,
    observation: UploadObservation,
) {
    for _ in 0..3 {
        let Ok(mut operation) = operations.load(&observation.operation_id).await else {
            return;
        };
        let expected_revision = operation.revision();
        if !operation
            .observe_progress(observation.clone())
            .unwrap_or(false)
        {
            return;
        }
        operation.next_event_sequence();
        match operations
            .compare_and_swap(expected_revision, operation)
            .await
        {
            Ok(committed) => {
                events.emit(TranscriptionService::event(&committed));
                return;
            }
            Err(RepositoryError::RevisionConflict) => continue,
            Err(_) => return,
        }
    }
}
