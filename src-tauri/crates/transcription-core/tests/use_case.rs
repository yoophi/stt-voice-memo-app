use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
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
            .filter(|item| !item.phase().is_terminal())
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
    local_cancelled: AtomicBool,
}

impl Backend {
    fn new(state: BackendState) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            state: Mutex::new(state),
            local_cancelled: AtomicBool::new(false),
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
        Ok(BackendOperation::active(
            BackendOperationId::parse("backend-1").unwrap(),
            request.source.id,
            *self.state.lock().unwrap(),
        ))
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
        }
        Ok(operation)
    }
    async fn delete(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls.lock().unwrap().push("delete");
        let mut result = BackendOperation::active(
            request.backend_operation_id,
            SourceAudioId::parse("source-1").unwrap(),
            BackendState::Cancelled,
        );
        result.cleanup = CleanupDisposition::Completed;
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
fn submit_deduplicates_then_status_returns_one_nonblank_final_result() {
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
        assert_eq!(backend.calls.lock().unwrap().as_slice(), ["create", "get"]);

        *backend.state.lock().unwrap() = BackendState::Completed;
        let completed = service.status(first.operation.id().clone()).await.unwrap();
        assert_eq!(completed.operation.phase(), OperationPhase::Completed);
        assert_eq!(completed.transcript.unwrap().text(), "final text");
    });
}

#[test]
fn cancelling_an_upload_requests_transport_cancellation_and_keeps_reconciliation_pending() {
    block_on(async {
        let repository = Arc::new(MemoryRepository::default());
        let backend = Arc::new(Backend::new(BackendState::Queued));
        let mut candidate = TranscriptionOperation::new(
            TranscriptionOperationId::parse(LOCAL_ID).unwrap(),
            SourceAudioId::parse("source-1").unwrap(),
            SubmissionFingerprint::parse(&"a".repeat(64)).unwrap(),
            TranscriptionOptions::default(),
        );
        candidate.begin_upload(100).unwrap();
        let stored = repository.get_or_create(candidate).await.unwrap().operation;
        let service = service(repository, backend.clone());

        let outcome = service.cancel(stored.id().clone()).await.unwrap();
        assert_eq!(outcome.operation.phase(), OperationPhase::Cancelling);
        assert!(outcome.operation.cancel_requested());
        assert!(backend.local_cancelled.load(Ordering::SeqCst));
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
            backend,
            Arc::new(FixtureSource),
            repository,
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
        let service = service(repository, backend.clone());
        let submitted = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        let cancelled = service
            .cancel(submitted.operation.id().clone())
            .await
            .unwrap();
        assert_eq!(
            cancelled.operation.terminal_winner(),
            Some(TerminalWinner::Cancelled)
        );
        assert_eq!(
            backend.calls.lock().unwrap().as_slice(),
            ["create", "delete"]
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
