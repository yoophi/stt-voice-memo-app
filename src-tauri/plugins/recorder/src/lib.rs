use std::sync::Mutex;

use recorder_core::{
    CleanupOutcome, FinalizedRecording, PermissionOutcome, RecorderService, RecordingSession,
    RecordingSessionId,
};
use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

mod commands;
#[cfg(desktop)]
mod desktop;
mod error;
#[cfg(mobile)]
mod mobile;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::{PlatformRecorder, init as init_platform};
#[cfg(mobile)]
use mobile::{PlatformRecorder, init as init_platform};
use models::{SessionRequest, StatusRequest, StopRequest};

pub struct Recorder<R: Runtime> {
    service: Mutex<RecorderService<PlatformRecorder<R>>>,
}

impl<R: Runtime> Recorder<R> {
    fn with_service<T>(
        &self,
        operation: impl FnOnce(&mut RecorderService<PlatformRecorder<R>>) -> Result<T>,
    ) -> Result<T> {
        let mut service = self.service.lock().map_err(|_| {
            recorder_core::RecorderError::new(
                recorder_core::RecorderErrorCode::RecorderFailure,
                None,
                true,
            )
        })?;
        operation(&mut service)
    }

    fn permission_status(&self) -> Result<PermissionOutcome> {
        self.with_service(|service| service.permission_status().map_err(Into::into))
    }

    fn request_permission(&self) -> Result<PermissionOutcome> {
        self.with_service(|service| service.request_permission().map_err(Into::into))
    }

    fn status(&self, payload: StatusRequest) -> Result<RecordingSession> {
        let session_id = payload
            .session_id
            .as_deref()
            .map(RecordingSessionId::parse)
            .transpose()?;
        self.with_service(|service| service.status(session_id.as_ref()).map_err(Into::into))
    }

    fn start(&self, payload: SessionRequest) -> Result<RecordingSession> {
        let session_id = RecordingSessionId::parse(&payload.session_id)?;
        self.with_service(|service| service.start(session_id).map_err(Into::into))
    }

    fn pause(&self, payload: SessionRequest) -> Result<RecordingSession> {
        let session_id = RecordingSessionId::parse(&payload.session_id)?;
        self.with_service(|service| service.pause(&session_id).map_err(Into::into))
    }

    fn resume(&self, payload: SessionRequest) -> Result<RecordingSession> {
        let session_id = RecordingSessionId::parse(&payload.session_id)?;
        self.with_service(|service| service.resume(&session_id).map_err(Into::into))
    }

    fn stop(&self, payload: StopRequest) -> Result<FinalizedRecording> {
        let session_id = RecordingSessionId::parse(&payload.session_id)?;
        self.with_service(|service| {
            service
                .stop(&session_id, payload.reason)
                .map_err(Into::into)
        })
    }

    fn cancel(&self, payload: SessionRequest) -> Result<CleanupOutcome> {
        let session_id = RecordingSessionId::parse(&payload.session_id)?;
        self.with_service(|service| service.cancel(&session_id).map_err(Into::into))
    }
}

pub trait RecorderExt<R: Runtime> {
    fn recorder(&self) -> &Recorder<R>;
}

impl<R: Runtime, T: Manager<R>> RecorderExt<R> for T {
    fn recorder(&self) -> &Recorder<R> {
        self.state::<Recorder<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("recorder")
        .invoke_handler(tauri::generate_handler![
            commands::permission_status,
            commands::request_permission,
            commands::recorder_status,
            commands::start,
            commands::pause,
            commands::resume,
            commands::stop,
            commands::cancel,
        ])
        .setup(|app, api| {
            let platform = init_platform(app, api)?;
            app.manage(Recorder {
                service: Mutex::new(RecorderService::new(platform)),
            });
            Ok(())
        })
        .build()
}
