use tauri::{AppHandle, Emitter};
use transcription_core::{CleanupDisposition, OperationEvent, OperationEventSink, OperationPhase};

const EVENT_NAME: &str = "transcription://event";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeOperationEvent {
    pub event_id: String,
    pub operation_id: String,
    pub sequence: u64,
    pub attempt: u32,
    pub phase: OperationPhase,
    pub progress_basis_points: Option<u16>,
    pub failure_code: Option<String>,
    pub retry_at_ms: Option<u64>,
    pub cleanup: CleanupDisposition,
}

impl From<OperationEvent> for SafeOperationEvent {
    fn from(event: OperationEvent) -> Self {
        Self {
            event_id: format!("{}:{}", event.operation_id, event.sequence),
            operation_id: event.operation_id.to_string(),
            sequence: event.sequence,
            attempt: event.attempt,
            phase: event.phase,
            progress_basis_points: event.progress_basis_points,
            failure_code: event.failure_code,
            retry_at_ms: event.retry_at_ms,
            cleanup: event.cleanup,
        }
    }
}

pub struct TauriOperationEventSink {
    app: AppHandle,
}

impl TauriOperationEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl OperationEventSink for TauriOperationEventSink {
    fn emit(&self, event: OperationEvent) {
        let _ = self.app.emit(EVENT_NAME, SafeOperationEvent::from(event));
    }
}

#[cfg(test)]
mod tests {
    use transcription_core::{
        CleanupDisposition, OperationEvent, OperationPhase, TranscriptionOperationId,
    };

    use super::SafeOperationEvent;

    #[test]
    fn event_serialization_excludes_sensitive_content_canaries() {
        let event = SafeOperationEvent::from(OperationEvent {
            operation_id: TranscriptionOperationId::new(),
            sequence: 3,
            attempt: 1,
            phase: OperationPhase::Processing,
            progress_basis_points: Some(5_000),
            failure_code: None,
            retry_at_ms: None,
            cleanup: CleanupDisposition::NotScheduled,
        });
        let json = serde_json::to_string(&event).unwrap();
        for forbidden in [
            "authorization",
            "transcript",
            "storagePath",
            "signedUrl",
            "providerPayload",
            "audioBytes",
            "CANARY_SECRET",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
