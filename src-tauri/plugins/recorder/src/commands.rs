use recorder_core::{CleanupOutcome, FinalizedRecording, PermissionOutcome, RecordingSession};
use tauri::{AppHandle, Runtime, command};

use crate::RecorderExt;
use crate::error::Result;
use crate::models::{SessionRequest, StatusRequest, StopRequest};

#[command]
pub(crate) fn permission_status<R: Runtime>(app: AppHandle<R>) -> Result<PermissionOutcome> {
    app.recorder().permission_status()
}

#[command]
pub(crate) fn request_permission<R: Runtime>(app: AppHandle<R>) -> Result<PermissionOutcome> {
    app.recorder().request_permission()
}

#[command]
pub(crate) fn recorder_status<R: Runtime>(
    app: AppHandle<R>,
    payload: StatusRequest,
) -> Result<RecordingSession> {
    app.recorder().status(payload)
}

#[command]
pub(crate) fn start<R: Runtime>(
    app: AppHandle<R>,
    payload: SessionRequest,
) -> Result<RecordingSession> {
    app.recorder().start(payload)
}

#[command]
pub(crate) fn pause<R: Runtime>(
    app: AppHandle<R>,
    payload: SessionRequest,
) -> Result<RecordingSession> {
    app.recorder().pause(payload)
}

#[command]
pub(crate) fn resume<R: Runtime>(
    app: AppHandle<R>,
    payload: SessionRequest,
) -> Result<RecordingSession> {
    app.recorder().resume(payload)
}

#[command]
pub(crate) fn stop<R: Runtime>(
    app: AppHandle<R>,
    payload: StopRequest,
) -> Result<FinalizedRecording> {
    app.recorder().stop(payload)
}

#[command]
pub(crate) fn cancel<R: Runtime>(
    app: AppHandle<R>,
    payload: SessionRequest,
) -> Result<CleanupOutcome> {
    app.recorder().cancel(payload)
}
