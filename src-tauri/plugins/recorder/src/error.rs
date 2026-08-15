use recorder_core::RecorderError;
#[cfg(target_os = "ios")]
use recorder_core::RecordingSessionId;
#[cfg(any(target_os = "ios", test))]
use recorder_core::{CleanupOutcome, RecorderErrorCode};
use serde::ser::{Serialize, SerializeStruct, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Recorder(#[from] RecorderError),
}

impl Error {
    #[cfg(target_os = "ios")]
    pub(crate) fn from_mobile(
        error: tauri::plugin::mobile::PluginInvokeError,
        session_id: Option<RecordingSessionId>,
    ) -> Self {
        let (code, cleanup) = match error {
            tauri::plugin::mobile::PluginInvokeError::InvokeRejected(response) => (
                parse_error_code(response.code.as_deref()),
                parse_cleanup(response.code.as_deref()),
            ),
            _ => (RecorderErrorCode::NativeCommandFailed, None),
        };
        let retryable = matches!(
            code,
            RecorderErrorCode::StorageUnavailable
                | RecorderErrorCode::AudioSessionFailure
                | RecorderErrorCode::RecorderFailure
                | RecorderErrorCode::CleanupFailure
                | RecorderErrorCode::NativeCommandFailed
        );
        let mut error = RecorderError::new(code, session_id, retryable);
        if let Some(cleanup) = cleanup {
            error = error.with_cleanup(cleanup);
        }
        error.into()
    }

    fn recorder(&self) -> &RecorderError {
        match self {
            Self::Recorder(error) => error,
        }
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let error = self.recorder();
        let mut state = serializer.serialize_struct("RecorderError", 5)?;
        state.serialize_field("code", &error.code)?;
        state.serialize_field("message", error.public_message())?;
        state.serialize_field("sessionId", &error.session_id)?;
        state.serialize_field("retryable", &error.retryable)?;
        state.serialize_field("cleanup", &error.cleanup)?;
        state.end()
    }
}

#[cfg(any(target_os = "ios", test))]
fn parse_error_code(code: Option<&str>) -> RecorderErrorCode {
    match code.and_then(|value| value.split(':').next()) {
        Some("invalidSessionId") => RecorderErrorCode::InvalidSessionId,
        Some("activeSessionExists") => RecorderErrorCode::ActiveSessionExists,
        Some("invalidTransition") => RecorderErrorCode::InvalidTransition,
        Some("staleSession") => RecorderErrorCode::StaleSession,
        Some("permissionDenied") => RecorderErrorCode::PermissionDenied,
        Some("permissionRestricted") => RecorderErrorCode::PermissionRestricted,
        Some("permissionRequestUnavailable") => RecorderErrorCode::PermissionRequestUnavailable,
        Some("storageUnavailable") => RecorderErrorCode::StorageUnavailable,
        Some("audioSessionFailure") => RecorderErrorCode::AudioSessionFailure,
        Some("recorderFailure") => RecorderErrorCode::RecorderFailure,
        Some("finalizationFailure") => RecorderErrorCode::FinalizationFailure,
        Some("invalidArtifact") => RecorderErrorCode::InvalidArtifact,
        Some("cleanupFailure") => RecorderErrorCode::CleanupFailure,
        Some("terminalConflict") => RecorderErrorCode::TerminalConflict,
        Some("unsupportedPlatform") => RecorderErrorCode::UnsupportedPlatform,
        _ => RecorderErrorCode::NativeCommandFailed,
    }
}

#[cfg(any(target_os = "ios", test))]
fn parse_cleanup(code: Option<&str>) -> Option<CleanupOutcome> {
    match code.and_then(|value| value.split_once(':').map(|(_, cleanup)| cleanup)) {
        Some("removed") => Some(CleanupOutcome::Removed),
        Some("notFound") => Some(CleanupOutcome::NotFound),
        Some("pending") => Some(CleanupOutcome::Pending),
        Some("failed") => Some(CleanupOutcome::Failed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use recorder_core::{RecorderError, RecorderErrorCode};

    use super::*;

    #[test]
    fn serialized_error_is_stable_and_contains_no_native_message() {
        let error = Error::Recorder(RecorderError::new(
            RecorderErrorCode::NativeCommandFailed,
            None,
            true,
        ));
        let value = serde_json::to_value(error).unwrap();
        assert_eq!(value["code"], "nativeCommandFailed");
        assert_eq!(value["message"], "native recorder command failed");
        assert_eq!(value["retryable"], true);
        assert!(value.get("nativeMessage").is_none());
    }

    #[test]
    fn internal_native_code_normalizes_cleanup_without_exposing_details() {
        assert_eq!(
            parse_error_code(Some("cleanupFailure:pending")),
            RecorderErrorCode::CleanupFailure
        );
        assert_eq!(
            parse_cleanup(Some("cleanupFailure:pending")),
            Some(CleanupOutcome::Pending)
        );
        assert_eq!(
            parse_error_code(Some("finalizationFailure:failed")),
            RecorderErrorCode::FinalizationFailure
        );
        assert_eq!(
            parse_cleanup(Some("finalizationFailure:failed")),
            Some(CleanupOutcome::Failed)
        );
    }
}
