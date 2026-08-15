use std::sync::Arc;

use thiserror::Error;

use crate::{
    AuthorizationError, AuthorizationPort, BackendOperation, BackendOperationRequest, BackendState,
    CleanupDisposition, Clock, ConnectivityPort, CreateTranscriptionRequest, DomainError, Failure,
    FailureCategory, FinalTranscript, OperationEvent, OperationEventSink, OperationPhase,
    OperationRepository, RepositoryError, SourceAudioError, SourceAudioId, SourceAudioPort,
    SubmissionFingerprint, TranscriptionOperation, TranscriptionOperationId, TranscriptionOptions,
    TranscriptionPort, TranscriptionPortError, UploadObservation, UploadProgressSink,
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
        let authorization = self.authorization.acquire().await?;
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
            OperationPhase::Cancelling | OperationPhase::CleanupPending
                if current.backend_operation_id().is_some() =>
            {
                return self.dispatch_delete(current).await;
            }
            OperationPhase::Uploading => {
                debug_assert!(current.recover_interrupted_upload());
                current = self.commit(current).await?;
            }
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
        if !self.connectivity.is_online().await {
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
            Err(error) => {
                let failure = Failure::new(
                    "AUTHENTICATION_REQUIRED",
                    FailureCategory::UserActionable,
                    None,
                )?;
                operation.fail(failure, self.clock.now_ms())?;
                let _ = self.commit(operation).await?;
                return Err(ApplicationError::Authorization(error));
            }
        };
        let request = CreateTranscriptionRequest {
            operation_id: operation.id().clone(),
            source,
            fingerprint: operation.fingerprint().clone(),
            options: operation.options().clone(),
            attempt: operation.attempt(),
            authorization,
            progress: Arc::new(NoopProgress),
        };
        match self.backend.create(request).await {
            Ok(remote) => self.apply_backend(operation, remote).await,
            Err(error) => self.apply_port_failure(operation, error).await,
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

    async fn apply_port_failure(
        &self,
        mut operation: TranscriptionOperation,
        error: TranscriptionPortError,
    ) -> Result<OperationOutcome, ApplicationError> {
        if operation.terminal_winner().is_some() {
            return Ok(OperationOutcome::operation(operation));
        }
        operation.fail(error.failure, self.clock.now_ms())?;
        Ok(OperationOutcome::operation(self.commit(operation).await?))
    }

    async fn apply_backend(
        &self,
        mut operation: TranscriptionOperation,
        remote: BackendOperation,
    ) -> Result<OperationOutcome, ApplicationError> {
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
            if operation.phase() == OperationPhase::Completed
                && remote.state == BackendState::Completed
            {
                let transcript = self.final_transcript(&operation, &remote)?;
                return Ok(OperationOutcome {
                    operation,
                    transcript: Some(transcript),
                });
            }
            return Ok(OperationOutcome::operation(operation));
        }

        let transcript = match remote.state {
            BackendState::Queued => {
                operation.observe_backend_active(
                    remote.id,
                    OperationPhase::Queued,
                    remote.request_id,
                )?;
                None
            }
            BackendState::Processing => {
                operation.observe_backend_active(
                    remote.id,
                    OperationPhase::Processing,
                    remote.request_id,
                )?;
                None
            }
            BackendState::Completed => {
                let transcript = match self.final_transcript_with_ids(&operation, &remote) {
                    Ok(transcript) => transcript,
                    Err(_) => return self.malformed(operation).await,
                };
                operation.complete(Some(remote.id))?;
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
                )?;
                operation.fail(failure, self.clock.now_ms())?;
                None
            }
            BackendState::Cancelled | BackendState::Deleted => {
                operation.confirm_cancel(CleanupDisposition::Completed)?;
                None
            }
            BackendState::Deleting => {
                let cleanup = match remote.cleanup {
                    CleanupDisposition::NotScheduled | CleanupDisposition::Completed => {
                        CleanupDisposition::InProgress {
                            delete_by_ms: self.clock.now_ms(),
                        }
                    }
                    cleanup => cleanup,
                };
                operation.confirm_cancel(cleanup)?;
                None
            }
        };
        let operation = self.commit(operation).await?;
        Ok(OperationOutcome {
            operation,
            transcript,
        })
    }

    fn final_transcript(
        &self,
        operation: &TranscriptionOperation,
        remote: &BackendOperation,
    ) -> Result<FinalTranscript, ApplicationError> {
        self.final_transcript_with_ids(operation, remote)
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
            failure_code: operation.failure().map(|failure| failure.code.clone()),
            retry_at_ms: operation.retry().map(|retry| retry.earliest_retry_at_ms),
            cleanup: operation.cleanup().clone(),
        }
    }
}

struct NoopProgress;
impl UploadProgressSink for NoopProgress {
    fn observe(&self, _observation: UploadObservation) {}
}
