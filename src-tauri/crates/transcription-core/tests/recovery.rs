use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use async_trait::async_trait;
use futures::executor::block_on;
use transcription_core::*;

#[derive(Default)]
struct Repository {
    records: Mutex<HashMap<String, TranscriptionOperation>>,
    fail_next_cas: AtomicBool,
    conflict_replacement: Mutex<Option<TranscriptionOperation>>,
}

impl Repository {
    fn insert(&self, mut operation: TranscriptionOperation) -> TranscriptionOperation {
        operation.set_revision(1);
        self.records
            .lock()
            .unwrap()
            .insert(operation.id().to_string(), operation.clone());
        operation
    }
}

#[async_trait]
impl OperationRepository for Repository {
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
        if self.fail_next_cas.swap(false, Ordering::SeqCst) {
            return Err(RepositoryError::Unavailable);
        }
        let mut records = self.records.lock().unwrap();
        let current = records
            .get(&replacement.id().to_string())
            .ok_or(RepositoryError::NotFound)?;
        if current.revision() != expected {
            return Err(RepositoryError::RevisionConflict);
        }
        if let Some(mut conflict) = self.conflict_replacement.lock().unwrap().take() {
            conflict.set_revision(expected + 1);
            records.insert(conflict.id().to_string(), conflict);
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

struct Source;
#[async_trait]
impl SourceAudioPort for Source {
    async fn inspect(&self, id: &SourceAudioId) -> Result<SourceDescriptor, SourceAudioError> {
        SourceDescriptor::new(id.clone(), "audio/mp4", "m4a", 128, 1_000, "a".repeat(64))
            .map_err(|_| SourceAudioError::Invalid)
    }
}
struct Auth;
#[async_trait]
impl AuthorizationPort for Auth {
    async fn acquire(&self) -> Result<AccessToken, AuthorizationError> {
        AccessToken::new("token")
    }
}
struct Connectivity(AtomicBool);
#[async_trait]
impl ConnectivityPort for Connectivity {
    async fn is_online(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
struct Time(AtomicU64);
impl Clock for Time {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}
struct Events;
impl OperationEventSink for Events {
    fn emit(&self, _: OperationEvent) {}
}

#[derive(Default)]
struct Backend {
    create_results: Mutex<VecDeque<Result<BackendOperation, TranscriptionPortError>>>,
    get_results: Mutex<VecDeque<Result<BackendOperation, TranscriptionPortError>>>,
    delete_results: Mutex<VecDeque<Result<BackendOperation, TranscriptionPortError>>>,
    calls: Mutex<Vec<(String, String, u32)>>,
}

#[async_trait]
impl TranscriptionPort for Backend {
    async fn create(
        &self,
        request: CreateTranscriptionRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls.lock().unwrap().push((
            "create".into(),
            request.operation_id.to_string(),
            request.attempt,
        ));
        self.create_results.lock().unwrap().pop_front().unwrap()
    }
    async fn get(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls
            .lock()
            .unwrap()
            .push(("get".into(), request.operation_id.to_string(), 0));
        self.get_results.lock().unwrap().pop_front().unwrap()
    }
    async fn delete(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        self.calls
            .lock()
            .unwrap()
            .push(("delete".into(), request.operation_id.to_string(), 0));
        self.delete_results.lock().unwrap().pop_front().unwrap()
    }
}

fn failure(
    code: &str,
    category: FailureCategory,
    retry_after: Option<u64>,
) -> TranscriptionPortError {
    TranscriptionPortError {
        failure: Failure::new(code, category, retry_after).unwrap(),
    }
}

fn active(state: BackendState) -> BackendOperation {
    BackendOperation::active(
        BackendOperationId::parse("backend-1").unwrap(),
        SourceAudioId::parse("source-1").unwrap(),
        state,
    )
}

fn operation() -> TranscriptionOperation {
    let source = SourceDescriptor::new(
        SourceAudioId::parse("source-1").unwrap(),
        "audio/mp4",
        "m4a",
        128,
        1_000,
        "a".repeat(64),
    )
    .unwrap();
    let options = TranscriptionOptions::default();
    TranscriptionOperation::new(
        TranscriptionOperationId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        source.id.clone(),
        SubmissionFingerprint::derive(&source, &options),
        options,
    )
}

fn service(
    repository: Arc<Repository>,
    backend: Arc<Backend>,
    online: Arc<Connectivity>,
    clock: Arc<Time>,
) -> TranscriptionService {
    TranscriptionService::new(
        backend,
        Arc::new(Source),
        repository,
        Arc::new(Auth),
        online,
        clock,
        Arc::new(Events),
    )
}

#[test]
fn offline_submit_is_durable_and_recovery_never_dispatches() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        let service = service(
            repository,
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(false))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        let waiting = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(waiting.operation.phase(), OperationPhase::WaitingForNetwork);
        assert_eq!(
            service.recover().await.unwrap()[0].operation.id(),
            waiting.operation.id()
        );
        assert!(backend.calls.lock().unwrap().is_empty());
    });
}

#[test]
fn uncertain_create_replays_exact_identity_and_honors_retry_deadline() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        backend.create_results.lock().unwrap().extend([
            Err(failure(
                "RATE_LIMITED",
                FailureCategory::Retryable,
                Some(50),
            )),
            Ok(active(BackendState::Queued)),
        ]);
        let clock = Arc::new(Time(AtomicU64::new(100)));
        let service = service(
            repository,
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            clock.clone(),
        );
        let failed = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(failed.operation.phase(), OperationPhase::RetryableFailure);
        assert!(matches!(
            service.retry(failed.operation.id().clone()).await,
            Err(ApplicationError::Domain(DomainError::RetryNotReady))
        ));
        clock.0.store(150, Ordering::SeqCst);
        let retried = service.retry(failed.operation.id().clone()).await.unwrap();
        assert_eq!(retried.operation.phase(), OperationPhase::Queued);
        let calls = backend.calls.lock().unwrap();
        assert_eq!(calls[0].1, calls[1].1);
        assert_eq!((calls[0].2, calls[1].2), (1, 2));
    });
}

#[test]
fn uncertain_with_known_backend_id_resolves_get_before_any_create_replay() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        backend
            .create_results
            .lock()
            .unwrap()
            .push_back(Ok(active(BackendState::Queued)));
        backend.get_results.lock().unwrap().extend([
            Err(failure(
                "PROCESSING_TIMEOUT",
                FailureCategory::Uncertain,
                None,
            )),
            Ok(active(BackendState::Processing)),
        ]);
        let service = service(
            repository,
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        let queued = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        let uncertain = service.status(queued.operation.id().clone()).await.unwrap();
        assert_eq!(uncertain.operation.phase(), OperationPhase::Uncertain);
        assert_eq!(
            service
                .retry(queued.operation.id().clone())
                .await
                .unwrap()
                .operation
                .phase(),
            OperationPhase::Processing
        );
        assert_eq!(
            backend
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|call| call.0.as_str())
                .collect::<Vec<_>>(),
            ["create", "get", "get"]
        );
    });
}

#[test]
fn recovery_projects_interrupted_upload_to_uncertain_without_network() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        let mut interrupted = operation();
        interrupted.begin_upload(100).unwrap();
        let stored = repository.insert(interrupted);
        let service = service(
            repository,
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        let recovered = service.recover().await.unwrap();
        assert_eq!(recovered[0].operation.id(), stored.id());
        assert_eq!(recovered[0].operation.phase(), OperationPhase::Uncertain);
        assert!(backend.calls.lock().unwrap().is_empty());
    });
}

#[test]
fn persistence_failure_before_upload_forbids_backend_side_effect() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        repository.fail_next_cas.store(true, Ordering::SeqCst);
        let backend = Arc::new(Backend::default());
        let service = service(
            repository,
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        assert!(matches!(
            service
                .submit(
                    SourceAudioId::parse("source-1").unwrap(),
                    TranscriptionOptions::default()
                )
                .await,
            Err(ApplicationError::Repository(RepositoryError::Unavailable))
        ));
        assert!(backend.calls.lock().unwrap().is_empty());
    });
}

#[test]
fn completion_winning_cancel_cas_prevents_remote_delete() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        backend
            .create_results
            .lock()
            .unwrap()
            .push_back(Ok(active(BackendState::Queued)));
        let service = service(
            repository.clone(),
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        let queued = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        let mut completed = queued.operation.clone();
        completed
            .complete(Some(BackendOperationId::parse("backend-1").unwrap()))
            .unwrap();
        *repository.conflict_replacement.lock().unwrap() = Some(completed);

        assert!(matches!(
            service.cancel(queued.operation.id().clone()).await,
            Err(ApplicationError::TerminalConflict)
        ));
        assert_eq!(
            backend
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|call| call.0.as_str())
                .collect::<Vec<_>>(),
            ["create"]
        );
    });
}

#[test]
fn cancellation_winning_completion_cas_never_returns_transcript() {
    block_on(async {
        let repository = Arc::new(Repository::default());
        let backend = Arc::new(Backend::default());
        backend
            .create_results
            .lock()
            .unwrap()
            .push_back(Ok(active(BackendState::Queued)));
        let service = service(
            repository.clone(),
            backend.clone(),
            Arc::new(Connectivity(AtomicBool::new(true))),
            Arc::new(Time(AtomicU64::new(100))),
        );
        let queued = service
            .submit(
                SourceAudioId::parse("source-1").unwrap(),
                TranscriptionOptions::default(),
            )
            .await
            .unwrap();
        let mut completed_remote = active(BackendState::Completed);
        completed_remote.result = Some(BackendTranscript::new("must not escape", None));
        backend
            .get_results
            .lock()
            .unwrap()
            .push_back(Ok(completed_remote));
        let mut cancelled = queued.operation.clone();
        cancelled.cancel_local().unwrap();
        *repository.conflict_replacement.lock().unwrap() = Some(cancelled);

        let outcome = service.status(queued.operation.id().clone()).await.unwrap();
        assert_eq!(outcome.operation.phase(), OperationPhase::Cancelled);
        assert!(outcome.transcript.is_none());
    });
}
