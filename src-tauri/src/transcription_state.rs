use std::sync::Arc;

use async_trait::async_trait;
use tauri::{AppHandle, Manager};
use transcription_core::{
    BackendOperation, BackendOperationRequest, CreateTranscriptionRequest, Failure,
    FailureCategory, TranscriptionPort, TranscriptionPortError, TranscriptionService,
};

use crate::infrastructure::transcription::{
    auth_session::{OptimisticConnectivity, SystemClock, UnavailableAuthorization},
    http_backend::{CoreHttpTranscriptionPort, HttpBackendConfig, HttpTranscriptionBackend},
    local_operation_store::LocalOperationStore,
    private_source_audio::PrivateSourceAudioStore,
    tauri_event_sink::TauriOperationEventSink,
};

pub struct TranscriptionState {
    pub service: Arc<TranscriptionService>,
}

impl TranscriptionState {
    pub fn build(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let root = app.path().app_data_dir()?.join("transcription");
        let operations = Arc::new(LocalOperationStore::new(root.join("operations"))?);
        let sources = Arc::new(PrivateSourceAudioStore::new(root.join("source-audio"))?);
        let backend: Arc<dyn TranscriptionPort> = match std::env::var("STT_BACKEND_URL") {
            Ok(value) => {
                let url = url::Url::parse(&value)?;
                let client = HttpTranscriptionBackend::new(HttpBackendConfig::production(url))?;
                Arc::new(CoreHttpTranscriptionPort::new(client, sources.clone()))
            }
            Err(_) => Arc::new(UnconfiguredTranscriptionBackend),
        };
        let service = TranscriptionService::new(
            backend,
            sources,
            operations,
            Arc::new(UnavailableAuthorization),
            Arc::new(OptimisticConnectivity),
            Arc::new(SystemClock),
            Arc::new(TauriOperationEventSink::new(app.clone())),
        );
        Ok(Self {
            service: Arc::new(service),
        })
    }
}

/// Production endpoint/auth configuration belongs to the integration feature.
/// Keeping this fail-closed adapter avoids embedding credentials or endpoints.
struct UnconfiguredTranscriptionBackend;

#[async_trait]
impl TranscriptionPort for UnconfiguredTranscriptionBackend {
    async fn create(
        &self,
        _request: CreateTranscriptionRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        Err(unconfigured_failure())
    }

    async fn get(
        &self,
        _request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        Err(unconfigured_failure())
    }

    async fn delete(
        &self,
        _request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError> {
        Err(unconfigured_failure())
    }
}

fn unconfigured_failure() -> TranscriptionPortError {
    TranscriptionPortError {
        failure: Failure::new(
            "BACKEND_NOT_CONFIGURED",
            FailureCategory::UserActionable,
            None,
        )
        .expect("static failure is valid"),
    }
}
