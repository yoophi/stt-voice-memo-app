use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use async_trait::async_trait;
use futures::executor::block_on;
use transcription_core::*;

const LOCAL_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

#[derive(Default)]
struct MemoryRepository {
    records: Mutex<HashMap<String, TranscriptionOperation>>,
}

#[async_trait]
impl OperationRepository for MemoryRepository {
    async fn get_or_create(
        &self,
        mut candidate: TranscriptionOperation,
    ) -> Result<GetOrCreateResult, RepositoryError> {
        let mut records = self.records.lock().unwrap();
        if let Some(existing) = records
            .values()
            .find(|item| item.source_audio_id() == candidate.source_audio_id())
            .cloned()
        {
            return Ok(GetOrCreateResult {
                operation: existing,
                created: false,
            });
        }
        candidate.set_revision(1);
        records.insert(candidate.id().to_string(), candidate.clone());
        Ok(GetOrCreateResult {
            operation: candidate,
            created: true,
        })
    }
    async fn load(
        &self,
        id: &TranscriptionOperationId,
    ) -> Result<TranscriptionOperation, RepositoryError> {
        self.records
            .lock()
            .unwrap()
            .get(&id.to_string())
            .cloned()
            .ok_or(RepositoryError::NotFound)
    }
    async fn compare_and_swap(
        &self,
        expected: u64,
        mut replacement: TranscriptionOperation,
    ) -> Result<TranscriptionOperation, RepositoryError> {
        let mut records = self.records.lock().unwrap();
        let current = records
            .get(&replacement.id().to_string())
            .ok_or(RepositoryError::NotFound)?;
        if current.revision() != expected {
            return Err(RepositoryError::RevisionConflict);
        }
        replacement.set_revision(expected + 1);
        records.insert(replacement.id().to_string(), replacement.clone());
        Ok(replacement)
    }
    async fn list_unfinished(&self) -> Result<Vec<TranscriptionOperation>, RepositoryError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .values()
            .filter(|item| item.needs_recovery())
            .cloned()
            .collect())
    }
}

struct FixtureSource;
#[async_trait]
impl SourceAudioPort for FixtureSource {
    async fn inspect(&self, id: &SourceAudioId) -> Result<SourceDescriptor, SourceAudioError> {
        SourceDescriptor::new(id.clone(), "audio/mp4", "m4a", 128, 1_000, "a".repeat(64))
            .map_err(|_| SourceAudioError::Invalid)
    }
}

struct Auth;
#[async_trait]
impl AuthorizationPort for Auth {
    async fn acquire(&self) -> Result<AccessToken, AuthorizationError> {
        AccessToken::new("test-token")
    }
}
struct Online;
#[async_trait]
impl ConnectivityPort for Online {
    async fn is_online(&self) -> bool {
        true
    }
}
struct TestClock;
impl Clock for TestClock {
    fn now_ms(&self) -> u64 {
        100
    }
}
struct MutableClock(AtomicU64);
impl Clock for MutableClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
#[derive(Default)]
struct Events(Mutex<Vec<OperationEvent>>);
impl OperationEventSink for Events {
    fn emit(&self, event: OperationEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Backend {
    calls: Mutex<Vec<&'static str>>,
    state: Mutex<BackendState>,
    delete_state: Mutex<BackendState>,
    delete_backend_id: Mutex<Option<BackendOperationId>>,
    delete_failure: Mutex<Option<Failure>>,
    local_cancelled: AtomicBool,
    retained_progress: Mutex<Option<Arc<dyn UploadProgressSink>>>,
}

impl Backend {
    fn new(state: BackendState) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            state: Mutex::new(state),
            delete_state: Mutex::new(BackendState::Cancelled),
            delete_backend_id: Mutex::new(None),
            delete_failure: Mutex::new(None),
            local_cancelled: AtomicBool::new(false),
            retained_progress: Mutex::new(None),
        }
    }
}

#[async_trait]
impl TranscriptionPort for Backend {
    fn cancel_local(&self, _: &TranscriptionOperationId) -> bool {
        self.local_cancelled.store(true, Ordering::SeqCst);
        true
    }

    async fn create(
        &self,
        request: CreateTranscriptionRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls.lock().unwrap().push("create");
        assert_eq!(request.operation_id.to_string().len(), 36);
        request.progress.observe(
            UploadObservation::new(
                request.operation_id.clone(),
                request.attempt,
                1,
                request.source.byte_length,
                request.source.byte_length,
            )
            .unwrap(),
        );
        *self.retained_progress.lock().unwrap() = Some(request.progress.clone());
        let mut operation = BackendOperation::active(
            BackendOperationId::parse("backend-1").unwrap(),
            request.source.id,
            *self.state.lock().unwrap(),
        );
        operation.poll_after_ms = Some(2_000);
        Ok(operation)
    }
    async fn get(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls.lock().unwrap().push("get");
        let state = *self.state.lock().unwrap();
        let mut operation = BackendOperation::active(
            request.backend_operation_id,
            SourceAudioId::parse("source-1").unwrap(),
            state,
        );
        if state == BackendState::Completed {
            operation.result = Some(BackendTranscript::new(" final text ", Some("ko".into())));
            operation.cleanup = CleanupDisposition::Scheduled {
                delete_by_ms: 86_400_000,
            };
        }
        Ok(operation)
    }
    async fn delete(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls.lock().unwrap().push("delete");
        if let Some(failure) = self.delete_failure.lock().unwrap().take() {
            return Err(TranscriptionPortError { failure });
        }
        let mut result = BackendOperation::active(
            self.delete_backend_id
                .lock()
                .unwrap()
                .clone()
                .unwrap_or(request.backend_operation_id),
            SourceAudioId::parse("source-1").unwrap(),
            *self.delete_state.lock().unwrap(),
        );
        result.cleanup = CleanupDisposition::Completed;
        result.request_id = Some(BackendRequestId::parse("delete-request").unwrap());
        Ok(result)
    }
}

fn service(repository: Arc<MemoryRepository>, backend: Arc<Backend>) -> TranscriptionService {
    TranscriptionService::new(
        backend,
        Arc::new(FixtureSource),
        repository,
        Arc::new(Auth),
        Arc::new(Online),
        Arc::new(TestClock),
        Arc::new(Events::default()),
    )
}

#[test]
fn submit_does_not_wait_for_retained_progress_and_returns_one_final_result() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let service = service(repository, backend.clone());
        let source = SourceAudioId::parse("source-1").unwrap();

        let first = service
            .submit(source.clone(), TranscriptionOptions::default())
            .await
            .unwrap();
        let duplicate = service
            .submit(source, TranscriptionOptions::default())
            .await
            .unwrap();
        assert_eq!(first.operation.id(), duplicate.operation.id());
        assert_eq!(first.operation.progress().unwrap().supplied_bytes, 128);
        assert_eq!(first.operation.poll_at_ms(), Some(2_100));
        assert!(backend.retained_progress.lock().unwrap().is_some());
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["create", "get"]);

        *backend.state.lock().unwrap() = BackendState::Completed;
        let completed = service.status(first.operation.id().clone()).await.unwrap();
        assert_eq!(completed.operation.phase(), OperationPhase::Completed);
        assert_eq!(
            completed.operation.cleanup(),
            &CleanupDisposition::Scheduled {
                delete_by_ms: 86_400_000
            }
        );
        assert_eq!(completed.transcript.unwrap().text(), "final text");
    });
}

#[test]
fn cancelling_an_upload_stops_transport_then_replays_to_reconcile_delete() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let source = SourceDescriptor::new(
            SourceAudioId::parse("source-1").unwrap(),
            "audio/mp4",
            "m4a",
            128,
            1_000,
            "a".repeat(64),
        )
        .unwrap();
        let mut candidate = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            source.id.clone(),
            SubmissionFingerprint::derive(&source, &TranscriptionOptions::default()),
            TranscriptionOptions::default(),
        );
        candidate.begin_upload(100).unwrap();
        let stored = repository.get_or_create(candidate).await.unwrap().operation;
        let service = service(repository, backend.clone());

        let outcome = service.cancel(stored.id().clone()).await.unwrap();
        assert_eq!(outcome.operation.phase(), OperationPhase::Cancelling);
        assert!(outcome.operation.cancel_requested());
        assert!(backend.local_cancelled.load(Ordering::SeqCst));

        let reconciled = service.retry(stored.id().clone()).await.unwrap();
        assert_eq!(reconciled.operation.phase(), OperationPhase::Cancelled);
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["create", "delete"]
        );
    });
}

struct RefreshableAuth(AtomicBool);

#[async_trait]
impl AuthorizationPort for RefreshableAuth {
    async fn acquire(&self) -> Result<AccessToken, AuthorizationError> {
        if self.0.load(Ordering::SeqCst) {
            AccessToken::new("refreshed-token")
        } else {
            Err(AuthorizationError::Unavailable)
        }
    }
}

#[test]
fn authentication_required_remains_recoverable_after_token_refresh() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let authorization = Arc::new(RefreshableAuth(AtomicBool::new(false)));
        let service = TranscriptionService::new(
            backend.clone(),
            Arc::new(FixtureSource),
            repository.clone(),
            authorization.clone(),
            Arc::new(Online),
            Arc::new(TestClock),
            Arc::new(Events::default()),
        );

        assert!(matches!(
            service
                .submit(
                    SourceAudioId::parse("source-1").unwrap(),
                    TranscriptionOptions::default(),
                )
                .await,
            Err(ApplicationError::Authorization(_))
        ));
        let waiting = service.recover().await.unwrap().pop().unwrap();
        assert_eq!(
            waiting.operation.phase(),
            OperationPhase::WaitingForAuthorization
        );
        assert!(waiting.operation.terminal_winner().is_none());

        authorization.0.store(true, Ordering::SeqCst);
        let retried = service.retry(waiting.operation.id().clone()).await.unwrap();
        assert_eq!(retried.operation.phase(), OperationPhase::Queued);

        authorization.0.store(false, Ordering::SeqCst);
        assert!(matches!(
            service.status(retried.operation.id().clone()).await,
            Err(ApplicationError::Authorization(_))
        ));
        let waiting_after_status = service.recover().await.unwrap().pop().unwrap();
        assert_eq!(
            waiting_after_status.operation.phase(),
            OperationPhase::WaitingForAuthorization
        );
        authorization.0.store(true, Ordering::SeqCst);
        assert_eq!(
            service
                .retry(waiting_after_status.operation.id().clone())
                .await
                .unwrap()
                .operation
                .phase(),
            OperationPhase::Queued
        );

        *backend.state.lock().unwrap() = BackendState::Completed;
        let completed = service
            .status(waiting_after_status.operation.id().clone())
            .await
            .unwrap();
        assert_eq!(completed.operation.phase(), OperationPhase::Completed);
        authorization.0.store(false, Ordering::SeqCst);
        assert!(matches!(
            service.status(completed.operation.id().clone()).await,
            Err(ApplicationError::Authorization(_))
        ));
        let stored = repository.load(completed.operation.id()).await.unwrap();
        assert_eq!(stored.phase(), OperationPhase::Completed);
        assert_eq!(stored.terminal_winner(), Some(TerminalWinner::Completed));
    });
}

#[test]
fn terminal_failure_can_retry_unresolved_remote_cleanup_without_changing_winner() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let source = SourceDescriptor::new(
            SourceAudioId::parse("source-1").unwrap(),
            "audio/mp4",
            "m4a",
            128,
            1_000,
            "a".repeat(64),
        )
        .unwrap();
        let mut operation = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            source.id.clone(),
            SubmissionFingerprint::derive(&source, &TranscriptionOptions::default()),
            TranscriptionOptions::default(),
        );
        operation.begin_upload(100).unwrap();
        operation
            .observe_backend_active(
                BackendOperationId::parse("backend-1").unwrap(),
                OperationPhase::Processing,
                None,
                None,
            )
            .unwrap();
        operation
            .fail_terminal(Failure::new("INVALID_AUDIO", FailureCategory::Terminal, None).unwrap())
            .unwrap();
        operation.set_cleanup(CleanupDisposition::FailedRetrying {
            delete_by_ms: 1_000,
        });
        let stored = repository.get_or_create(operation).await.unwrap().operation;
        let events = Arc::new(Events::default());
        let clock = Arc::new(MutableClock(AtomicU64::new(100)));
        let service = TranscriptionService::new(
            backend.clone(),
            Arc::new(FixtureSource),
            repository,
            Arc::new(Auth),
            Arc::new(Online),
            clock.clone(),
            events.clone(),
        );

        let cleanup_failure = Failure::new(
            "BACKEND_UNAVAILABLE",
            FailureCategory::Retryable,
            Some(1_000),
        )
        .unwrap()
        .with_request_id(BackendRequestId::parse("cleanup-request").unwrap());
        *backend.delete_failure.lock().unwrap() = Some(cleanup_failure);

        let failed_cleanup = service.retry(stored.id().clone()).await.unwrap();
        assert_eq!(
            failed_cleanup.operation.phase(),
            OperationPhase::TerminalFailure
        );
        assert_eq!(
            failed_cleanup.operation.failure().unwrap().code,
            "INVALID_AUDIO"
        );
        assert_eq!(
            failed_cleanup.operation.cleanup_failure().unwrap().code,
            "BACKEND_UNAVAILABLE"
        );
        assert_eq!(
            failed_cleanup
                .operation
                .backend_request_id()
                .unwrap()
                .to_string(),
            "cleanup-request"
        );
        assert_eq!(
            failed_cleanup
                .operation
                .retry()
                .unwrap()
                .earliest_retry_at_ms,
            1_100
        );
        assert_eq!(
            events
                .0
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .failure_code
                .as_deref(),
            Some("BACKEND_UNAVAILABLE")
        );
        assert_eq!(service.recover().await.unwrap().len(), 1);

        let waiting = service
            .retry(failed_cleanup.operation.id().clone())
            .await
            .unwrap();
        assert_eq!(
            waiting.operation.cleanup_failure(),
            failed_cleanup.operation.cleanup_failure()
        );
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["delete"]);

        clock.0.store(1_100, Ordering::SeqCst);
        let cleaned = service.retry(waiting.operation.id().clone()).await.unwrap();
        assert_eq!(cleaned.operation.phase(), OperationPhase::TerminalFailure);
        assert_eq!(
            cleaned.operation.terminal_winner(),
            Some(TerminalWinner::TerminalFailure)
        );
        assert_eq!(cleaned.operation.cleanup(), &CleanupDisposition::Completed);
        assert!(cleaned.operation.cleanup_failure().is_none());
        assert_eq!(cleaned.operation.cleanup_attempts(), 2);
        assert_eq!(
            cleaned.operation.backend_request_id().unwrap().to_string(),
            "delete-request"
        );
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["delete", "delete"]
        );
    });
}

#[test]
fn terminal_cleanup_rejects_incompatible_remote_state_without_replacing_winner() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        *backend.delete_state.lock().unwrap() = BackendState::Queued;
        let source = SourceDescriptor::new(
            SourceAudioId::parse("source-1").unwrap(),
            "audio/mp4",
            "m4a",
            128,
            1_000,
            "a".repeat(64),
        )
        .unwrap();
        let mut operation = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            source.id.clone(),
            SubmissionFingerprint::derive(&source, &TranscriptionOptions::default()),
            TranscriptionOptions::default(),
        );
        operation.begin_upload(100).unwrap();
        operation
            .observe_backend_active(
                BackendOperationId::parse("backend-1").unwrap(),
                OperationPhase::Processing,
                None,
                None,
            )
            .unwrap();
        operation
            .fail_terminal(Failure::new("INVALID_AUDIO", FailureCategory::Terminal, None).unwrap())
            .unwrap();
        operation.set_cleanup(CleanupDisposition::FailedRetrying {
            delete_by_ms: 1_000,
        });
        let stored = repository.get_or_create(operation).await.unwrap().operation;
        let service = service(repository, backend.clone());

        let rejected = service.retry(stored.id().clone()).await.unwrap();
        assert_eq!(
            rejected.operation.terminal_winner(),
            Some(TerminalWinner::TerminalFailure)
        );
        assert_eq!(rejected.operation.failure().unwrap().code, "INVALID_AUDIO");
        assert_eq!(
            rejected.operation.cleanup_failure().unwrap().code,
            "MALFORMED_BACKEND_RESPONSE"
        );
        assert!(rejected.operation.retry().is_none());

        let unchanged = service
            .retry(rejected.operation.id().clone())
            .await
            .unwrap();
        assert_eq!(unchanged.operation, rejected.operation);
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["delete"]);
    });
}

#[test]
fn terminal_cleanup_persists_identity_mismatch_with_request_correlation() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        *backend.delete_backend_id.lock().unwrap() =
            Some(BackendOperationId::parse("different-backend").unwrap());
        let source = SourceDescriptor::new(
            SourceAudioId::parse("source-1").unwrap(),
            "audio/mp4",
            "m4a",
            128,
            1_000,
            "a".repeat(64),
        )
        .unwrap();
        let mut operation = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            source.id.clone(),
            SubmissionFingerprint::derive(&source, &TranscriptionOptions::default()),
            TranscriptionOptions::default(),
        );
        operation.begin_upload(100).unwrap();
        operation
            .observe_backend_active(
                BackendOperationId::parse("backend-1").unwrap(),
                OperationPhase::Processing,
                None,
                None,
            )
            .unwrap();
        operation
            .fail_terminal(Failure::new("INVALID_AUDIO", FailureCategory::Terminal, None).unwrap())
            .unwrap();
        operation.set_cleanup(CleanupDisposition::FailedRetrying {
            delete_by_ms: 1_000,
        });
        let stored = repository.get_or_create(operation).await.unwrap().operation;
        let service = service(repository, backend.clone());

        let rejected = service.retry(stored.id().clone()).await.unwrap();
        assert_eq!(
            rejected.operation.cleanup_failure().unwrap().code,
            "BACKEND_IDENTITY_MISMATCH"
        );
        assert_eq!(
            rejected.operation.backend_request_id().unwrap().to_string(),
            "delete-request"
        );
        assert_eq!(rejected.operation.cleanup_attempts(), 1);
        assert!(rejected.operation.retry().is_none());
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["delete"]);
    });
}

#[test]
fn cancel_before_dispatch_is_local_and_repeated_cancel_is_idempotent() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let candidate = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            SourceAudioId::parse("source-1").unwrap(),
            SubmissionFingerprint::parse(&"a".repeat(64)).unwrap(),
            TranscriptionOptions::default(),
        );
        let stored = repository.get_or_create(candidate).await.unwrap().operation;
        let service = service(repository, backend.clone());

        assert_eq!(
            service
                .cancel(stored.id().clone())
                .await
                .unwrap()
                .operation
                .phase(),
            OperationPhase::Cancelled
        );
        assert_eq!(
            service
                .cancel(stored.id().clone())
                .await
                .unwrap()
                .operation
                .phase(),
            OperationPhase::Cancelled
        );
        assert!(backend.calls.lock().unwrap().is_empty());
    });
}

#[test]
fn remote_cancel_persists_intent_and_preserves_completion_winner() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let clock = Arc::new(MutableClock(AtomicU64::new(100)));
        let service = TranscriptionService::new(
            backend.clone(),
            Arc::new(FixtureSource),
            repository,
            Arc::new(Auth),
            Arc::new(Online),
            clock.clone(),
            Arc::new(Events::default()),
        );
        let submitted = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        *backend.delete_failure.lock().unwrap() = Some(
            Failure::new(
                "BACKEND_UNAVAILABLE",
                FailureCategory::Retryable,
                Some(1_000),
            )
            .unwrap()
            .with_request_id(BackendRequestId::parse("cancel-failure").unwrap()),
        );
        let pending = service
            .cancel(submitted.operation.id().clone())
            .await
            .unwrap();
        assert_eq!(pending.operation.phase(), OperationPhase::CleanupPending);
        assert_eq!(pending.operation.cleanup_attempts(), 1);
        assert_eq!(
            pending.operation.retry().unwrap().earliest_retry_at_ms,
            1_100
        );
        let waiting = service.retry(pending.operation.id().clone()).await.unwrap();
        assert_eq!(waiting.operation.phase(), OperationPhase::CleanupPending);
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["create", "delete"]
        );

        clock.0.store(1_100, Ordering::SeqCst);
        let cancelled = service.retry(waiting.operation.id().clone()).await.unwrap();
        assert_eq!(
            cancelled.operation.terminal_winner(),
            Some(TerminalWinner::Cancelled)
        );
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["create", "delete", "delete"]
        );
    });
}

struct BlankBackend;
#[async_trait]
impl TranscriptionPort for BlankBackend {
    async fn create(
        &self,
        request: CreateTranscriptionRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        let mut remote = BackendOperation::active(
            BackendOperationId::parse("backend-1").unwrap(),
            request.source.id,
            BackendState::Completed,
        );
        remote.result = Some(BackendTranscript::new("   ", None));
        Ok(remote)
    }
    async fn get(
        &self,
        _: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        unreachable!()
    }
    async fn delete(
        &self,
        _: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        unreachable!()
    }
}

#[test]
fn blank_backend_success_is_persisted_as_terminal_malformed_response() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let service = TranscriptionService::new(
            Arc::new(BlankBackend),
            Arc::new(FixtureSource),
            repository,
            Arc::new(Auth),
            Arc::new(Online),
            Arc::new(TestClock),
            Arc::new(Events::default()),
        );
        let outcome = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(outcome.operation.phase(), OperationPhase::TerminalFailure);
        assert_eq!(
            outcome.operation.failure().unwrap().code,
            "MALFORMED_RESPONSE"
        );
        assert!(outcome.transcript.is_none());
    });
}
