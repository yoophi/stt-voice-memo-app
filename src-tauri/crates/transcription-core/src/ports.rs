use std::{fmt, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    BackendOperationId, BackendRequestId, CleanupDisposition, Failure, OperationPhase,
    SourceAudioId, SourceDescriptor, SubmissionFingerprint, TranscriptionOperation,
    TranscriptionOperationId, TranscriptionOptions, UploadObservation,
};

pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: impl Into<String>) -> Result<Self, AuthorizationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AuthorizationError::Unavailable);
        }
        Ok(Self(value))
    }

    pub fn expose_to_adapter(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AccessToken([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthorizationError {
    #[error("authorization is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SourceAudioError {
    #[error("source audio was not found")]
    NotFound,
    #[error("source audio is invalid")]
    Invalid,
    #[error("source audio is temporarily unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RepositoryError {
    #[error("operation was not found")]
    NotFound,
    #[error("operation revision conflict")]
    RevisionConflict,
    #[error("operation persistence is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetOrCreateResult {
    pub operation: TranscriptionOperation,
    pub created: bool,
}

#[async_trait]
pub trait SourceAudioPort: Send + Sync {
    async fn inspect(
        &self,
        source_id: &SourceAudioId,
    ) -> Result<SourceDescriptor, SourceAudioError>;
}

#[async_trait]
pub trait OperationRepository: Send + Sync {
    async fn get_or_create(
        &self,
        candidate: TranscriptionOperation,
    ) -> Result<GetOrCreateResult, RepositoryError>;

    async fn load(
        &self,
        operation_id: &TranscriptionOperationId,
    ) -> Result<TranscriptionOperation, RepositoryError>;

    async fn compare_and_swap(
        &self,
        expected_revision: u64,
        replacement: TranscriptionOperation,
    ) -> Result<TranscriptionOperation, RepositoryError>;

    async fn list_unfinished(&self) -> Result<Vec<TranscriptionOperation>, RepositoryError>;
}

#[async_trait]
pub trait AuthorizationPort: Send + Sync {
    async fn acquire(&self) -> Result<AccessToken, AuthorizationError>;
}

#[async_trait]
pub trait ConnectivityPort: Send + Sync {
    async fn is_online(&self) -> bool;
}

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub trait UploadProgressSink: Send + Sync {
    fn observe(&self, observation: UploadObservation);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendState {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Deleting,
    Deleted,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BackendTranscript {
    text: String,
    pub language: Option<String>,
}

impl BackendTranscript {
    pub fn new(text: impl Into<String>, language: Option<String>) -> Self {
        Self {
            text: text.into(),
            language,
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for BackendTranscript {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackendTranscript")
            .field("text", &"[REDACTED]")
            .field("language", &self.language)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendOperation {
    pub id: BackendOperationId,
    pub source_audio_id: SourceAudioId,
    pub state: BackendState,
    pub result: Option<BackendTranscript>,
    pub failure: Option<Failure>,
    pub cleanup: CleanupDisposition,
    pub request_id: Option<BackendRequestId>,
}

impl BackendOperation {
    pub fn active(
        id: BackendOperationId,
        source_audio_id: SourceAudioId,
        state: BackendState,
    ) -> Self {
        Self {
            id,
            source_audio_id,
            state,
            result: None,
            failure: None,
            cleanup: CleanupDisposition::NotScheduled,
            request_id: None,
        }
    }
}

pub struct CreateTranscriptionRequest {
    pub operation_id: TranscriptionOperationId,
    pub source: SourceDescriptor,
    pub fingerprint: SubmissionFingerprint,
    pub options: TranscriptionOptions,
    pub attempt: u32,
    pub authorization: AccessToken,
    pub progress: Arc<dyn UploadProgressSink>,
}

pub struct BackendOperationRequest {
    pub operation_id: TranscriptionOperationId,
    pub backend_operation_id: BackendOperationId,
    pub source_audio_id: SourceAudioId,
    pub authorization: AccessToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("backend operation failed: {failure:?}")]
pub struct TranscriptionPortError {
    pub failure: Failure,
}

#[async_trait]
pub trait TranscriptionPort: Send + Sync {
    fn cancel_local(&self, _operation_id: &TranscriptionOperationId) -> bool {
        false
    }

    async fn create(
        &self,
        request: CreateTranscriptionRequest,
    ) -> Result<BackendOperation, TranscriptionPortError>;

    async fn get(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError>;

    async fn delete(
        &self,
        request: BackendOperationRequest,
    ) -> Result<BackendOperation, TranscriptionPortError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub operation_id: TranscriptionOperationId,
    pub sequence: u64,
    pub attempt: u32,
    pub phase: OperationPhase,
    pub progress_basis_points: Option<u16>,
    pub failure_code: Option<String>,
    pub retry_at_ms: Option<u64>,
    pub cleanup: CleanupDisposition,
}

pub trait OperationEventSink: Send + Sync {
    fn emit(&self, event: OperationEvent);
}
