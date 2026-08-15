use serde::{Deserialize, Serialize};
use tauri::State;
use transcription_core::{
    ApplicationError, CleanupDisposition, FailureCategory, OperationOutcome, OperationPhase,
    SourceAudioId, TranscriptionOperationId, TranscriptionOptions,
};

use crate::transcription_state::TranscriptionState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitRequest {
    pub source_audio_id: String,
    pub language_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub operation_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    supplied_bytes: u64,
    total_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureView {
    code: String,
    category: FailureCategory,
    retryable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    operation_id: String,
    source_audio_id: String,
    phase: OperationPhase,
    attempt: u32,
    updated_at_ms: u64,
    progress: Option<ProgressView>,
    failure: Option<FailureView>,
    retry_at_ms: Option<u64>,
    cleanup: CleanupDisposition,
    backend_request_id: Option<String>,
    transcript: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    code: &'static str,
    category: FailureCategory,
    retryable: bool,
    operation_id: Option<String>,
    retry_at_ms: Option<u64>,
    backend_request_id: Option<String>,
}

#[tauri::command]
pub async fn transcription_submit(
    state: State<'_, TranscriptionState>,
    payload: SubmitRequest,
) -> Result<OperationView, CommandError> {
    let source_id = SourceAudioId::parse(payload.source_audio_id).map_err(invalid_request)?;
    let options =
        TranscriptionOptions::new(payload.language_hint.as_deref()).map_err(invalid_request)?;
    state
        .service
        .submit(source_id, options)
        .await
        .map(|outcome| OperationView::from_outcome(outcome, now_ms()))
        .map_err(command_error)
}

#[tauri::command]
pub async fn transcription_status(
    state: State<'_, TranscriptionState>,
    payload: OperationRequest,
) -> Result<OperationView, CommandError> {
    run_operation(state, payload, OperationAction::Status).await
}

#[tauri::command]
pub async fn transcription_retry(
    state: State<'_, TranscriptionState>,
    payload: OperationRequest,
) -> Result<OperationView, CommandError> {
    run_operation(state, payload, OperationAction::Retry).await
}

#[tauri::command]
pub async fn transcription_cancel(
    state: State<'_, TranscriptionState>,
    payload: OperationRequest,
) -> Result<OperationView, CommandError> {
    run_operation(state, payload, OperationAction::Cancel).await
}

#[tauri::command]
pub async fn transcription_recover(
    state: State<'_, TranscriptionState>,
) -> Result<Vec<OperationView>, CommandError> {
    state
        .service
        .recover()
        .await
        .map(|outcomes| {
            let updated_at_ms = now_ms();
            outcomes
                .into_iter()
                .map(|outcome| OperationView::from_outcome(outcome, updated_at_ms))
                .collect()
        })
        .map_err(command_error)
}

enum OperationAction {
    Status,
    Retry,
    Cancel,
}

async fn run_operation(
    state: State<'_, TranscriptionState>,
    payload: OperationRequest,
    action: OperationAction,
) -> Result<OperationView, CommandError> {
    let operation_id =
        TranscriptionOperationId::parse(&payload.operation_id).map_err(invalid_request)?;
    let outcome = match action {
        OperationAction::Status => state.service.status(operation_id).await,
        OperationAction::Retry => state.service.retry(operation_id).await,
        OperationAction::Cancel => state.service.cancel(operation_id).await,
    };
    outcome
        .map(|outcome| OperationView::from_outcome(outcome, now_ms()))
        .map_err(command_error)
}

impl OperationView {
    fn from_outcome(outcome: OperationOutcome, updated_at_ms: u64) -> Self {
        let operation = outcome.operation;
        Self {
            operation_id: operation.id().to_string(),
            source_audio_id: operation.source_audio_id().to_string(),
            phase: operation.phase(),
            attempt: operation.attempt(),
            updated_at_ms,
            progress: operation.progress().map(|progress| ProgressView {
                supplied_bytes: progress.supplied_bytes,
                total_bytes: progress.total_bytes,
            }),
            failure: operation.failure().map(|failure| FailureView {
                code: failure.code.clone(),
                category: failure.category,
                retryable: failure.retryable,
            }),
            retry_at_ms: operation
                .retry()
                .map(|retry| retry.earliest_retry_at_ms)
                .or(operation.poll_at_ms()),
            cleanup: operation.cleanup().clone(),
            backend_request_id: operation.backend_request_id().map(ToString::to_string),
            transcript: outcome
                .transcript
                .map(|transcript| transcript.text().to_owned()),
        }
    }
}

fn invalid_request<T>(_: T) -> CommandError {
    CommandError {
        code: "INVALID_REQUEST",
        category: FailureCategory::UserActionable,
        retryable: false,
        operation_id: None,
        retry_at_ms: None,
        backend_request_id: None,
    }
}

fn command_error(error: ApplicationError) -> CommandError {
    let code = match error {
        ApplicationError::Authorization(_) => "AUTHENTICATION_REQUIRED",
        ApplicationError::Repository(_) => "LOCAL_STATE_UNAVAILABLE",
        ApplicationError::Source(_) => "SOURCE_AUDIO_UNAVAILABLE",
        ApplicationError::IdempotencyMismatch => "IDEMPOTENCY_MISMATCH",
        ApplicationError::TerminalConflict => "TERMINAL_CONFLICT",
        ApplicationError::Domain(_) => "INVALID_OPERATION_STATE",
    };
    CommandError {
        code,
        category: FailureCategory::UserActionable,
        retryable: false,
        operation_id: None,
        retry_at_ms: None,
        backend_request_id: None,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_request_rejects_adapter_owned_fields() {
        let value = serde_json::json!({
            "sourceAudioId": "source-one",
            "backendUrl": "https://forbidden.example",
        });
        assert!(serde_json::from_value::<SubmitRequest>(value).is_err());
    }

    #[test]
    fn command_error_is_content_safe() {
        let json = serde_json::to_string(&invalid_request(())).unwrap();
        for forbidden in ["token", "path", "transcript", "provider", "audio"] {
            assert!(!json.to_ascii_lowercase().contains(forbidden));
        }
    }
}
