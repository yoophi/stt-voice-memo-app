use std::fs::File;
use std::io::Read;
use std::path::Path;

use recorder_core::{
    ArtifactId, CleanupOutcome, FinalizationReason, FinalizedRecording, PermissionOutcome,
    RecorderError, RecorderErrorCode, RecorderPort, RecordingSession, RecordingSessionId,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
#[cfg(target_os = "ios")]
use tauri::plugin::PluginHandle;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

#[cfg(target_os = "ios")]
use crate::error::Error;
use crate::models::{NativeFinalizedRecording, SessionRequest, StatusRequest, StopRequest};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_recorder);

#[cfg(target_os = "ios")]
pub(crate) struct PlatformRecorder<R: Runtime>(PluginHandle<R>);

#[cfg(target_os = "android")]
pub(crate) struct PlatformRecorder<R: Runtime>(std::marker::PhantomData<fn() -> R>);

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::error::Result<PlatformRecorder<R>> {
    #[cfg(target_os = "ios")]
    let handle = api
        .register_ios_plugin(init_plugin_recorder)
        .map_err(|error| Error::from_mobile(error, None))?;
    #[cfg(target_os = "ios")]
    return Ok(PlatformRecorder(handle));

    #[cfg(target_os = "android")]
    {
        let _ = api;
        Ok(PlatformRecorder(std::marker::PhantomData))
    }
}

impl<R: Runtime> PlatformRecorder<R> {
    #[cfg(target_os = "ios")]
    fn invoke<T: DeserializeOwned, P: serde::Serialize>(
        &self,
        command: &str,
        payload: P,
        session_id: Option<RecordingSessionId>,
    ) -> Result<T, RecorderError> {
        self.0.run_mobile_plugin(command, payload).map_err(|error| {
            match Error::from_mobile(error, session_id) {
                Error::Recorder(error) => error,
            }
        })
    }

    #[cfg(target_os = "android")]
    fn invoke<T: DeserializeOwned, P: serde::Serialize>(
        &self,
        _command: &str,
        _payload: P,
        session_id: Option<RecordingSessionId>,
    ) -> Result<T, RecorderError> {
        Err(RecorderError::new(
            RecorderErrorCode::UnsupportedPlatform,
            session_id,
            false,
        ))
    }
}

impl<R: Runtime> RecorderPort for PlatformRecorder<R> {
    fn permission_status(&mut self) -> Result<PermissionOutcome, RecorderError> {
        self.invoke("permission_status", (), None)
    }

    fn request_permission(&mut self) -> Result<PermissionOutcome, RecorderError> {
        self.invoke("request_permission", (), None)
    }

    fn status(
        &mut self,
        session_id: Option<&RecordingSessionId>,
    ) -> Result<RecordingSession, RecorderError> {
        self.invoke(
            "recorder_status",
            StatusRequest {
                session_id: session_id.map(ToString::to_string),
            },
            session_id.cloned(),
        )
    }

    fn start(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        self.invoke(
            "start",
            SessionRequest {
                session_id: session_id.to_string(),
            },
            Some(session_id.clone()),
        )
    }

    fn pause(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        self.invoke(
            "pause",
            SessionRequest {
                session_id: session_id.to_string(),
            },
            Some(session_id.clone()),
        )
    }

    fn resume(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        self.invoke(
            "resume",
            SessionRequest {
                session_id: session_id.to_string(),
            },
            Some(session_id.clone()),
        )
    }

    fn stop(
        &mut self,
        session_id: &RecordingSessionId,
        reason: FinalizationReason,
    ) -> Result<FinalizedRecording, RecorderError> {
        let native: NativeFinalizedRecording = self.invoke(
            "stop",
            StopRequest {
                session_id: session_id.to_string(),
                reason,
            },
            Some(session_id.clone()),
        )?;
        validate_native_recording(session_id.clone(), native)
    }

    fn cancel(&mut self, session_id: &RecordingSessionId) -> Result<CleanupOutcome, RecorderError> {
        self.invoke(
            "cancel",
            SessionRequest {
                session_id: session_id.to_string(),
            },
            Some(session_id.clone()),
        )
    }
}

fn validate_native_recording(
    session_id: RecordingSessionId,
    native: NativeFinalizedRecording,
) -> Result<FinalizedRecording, RecorderError> {
    let invalid = || {
        RecorderError::new(
            RecorderErrorCode::InvalidArtifact,
            Some(session_id.clone()),
            false,
        )
    };
    let url = url::Url::parse(&native.file_uri).map_err(|_| invalid())?;
    let path = url.to_file_path().map_err(|_| invalid())?;
    if path.extension().and_then(|value| value.to_str()) != Some("m4a")
        || !owned_recordings_path(&path)
    {
        return Err(invalid());
    }
    let metadata = std::fs::metadata(&path).map_err(|_| invalid())?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() != native.byte_length
        || native.session_id != session_id.to_string()
    {
        return Err(invalid());
    }
    let mut file = File::open(&path).map_err(|_| invalid())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| invalid())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let checksum = format!("{:x}", hasher.finalize());
    if checksum != native.sha256 {
        return Err(invalid());
    }
    FinalizedRecording::new(
        ArtifactId::parse(&native.artifact_id).map_err(|_| invalid())?,
        session_id,
        "audio/mp4",
        "m4a",
        native.duration_ms,
        native.byte_length,
        native.sample_rate_hz,
        native.channel_count,
        checksum,
        native.finalization_reason,
    )
}

fn owned_recordings_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .parent()
            .is_some_and(|parent| parent.ends_with("Library/Application Support/Recordings"))
}
