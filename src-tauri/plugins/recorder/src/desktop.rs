use recorder_core::{
    CleanupOutcome, FinalizationReason, FinalizedRecording, PermissionOutcome, RecorderError,
    RecorderPort, RecordingSession, RecordingSessionId,
};
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

pub(crate) struct PlatformRecorder<R: Runtime>(#[allow(dead_code)] AppHandle<R>);

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::error::Result<PlatformRecorder<R>> {
    Ok(PlatformRecorder(app.clone()))
}

impl<R: Runtime> RecorderPort for PlatformRecorder<R> {
    fn permission_status(&mut self) -> Result<PermissionOutcome, RecorderError> {
        Err(unsupported())
    }

    fn request_permission(&mut self) -> Result<PermissionOutcome, RecorderError> {
        Err(unsupported())
    }

    fn status(
        &mut self,
        _session_id: Option<&RecordingSessionId>,
    ) -> Result<RecordingSession, RecorderError> {
        Err(unsupported())
    }

    fn start(
        &mut self,
        _session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        Err(unsupported())
    }

    fn pause(
        &mut self,
        _session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        Err(unsupported())
    }

    fn resume(
        &mut self,
        _session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        Err(unsupported())
    }

    fn stop(
        &mut self,
        _session_id: &RecordingSessionId,
        _reason: FinalizationReason,
    ) -> Result<FinalizedRecording, RecorderError> {
        Err(unsupported())
    }

    fn cancel(
        &mut self,
        _session_id: &RecordingSessionId,
    ) -> Result<CleanupOutcome, RecorderError> {
        Err(unsupported())
    }
}

fn unsupported() -> RecorderError {
    RecorderError::new(
        recorder_core::RecorderErrorCode::UnsupportedPlatform,
        None,
        false,
    )
}
