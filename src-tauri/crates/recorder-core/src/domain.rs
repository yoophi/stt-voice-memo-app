use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordingSessionId(Uuid);

impl RecordingSessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, RecorderError> {
        let parsed = Uuid::parse_str(value)
            .map_err(|_| RecorderError::new(RecorderErrorCode::InvalidSessionId, None, false))?;
        if parsed.to_string() != value {
            return Err(RecorderError::new(
                RecorderErrorCode::InvalidSessionId,
                None,
                false,
            ));
        }
        Ok(Self(parsed))
    }
}

impl Default for RecordingSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RecordingSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactId(Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, RecorderError> {
        let parsed = Uuid::parse_str(value)
            .map_err(|_| RecorderError::new(RecorderErrorCode::InvalidArtifact, None, false))?;
        if parsed.to_string() != value {
            return Err(RecorderError::new(
                RecorderErrorCode::InvalidArtifact,
                None,
                false,
            ));
        }
        Ok(Self(parsed))
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    Undetermined,
    Granted,
    Denied,
    Restricted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOutcome {
    pub state: PermissionState,
    pub can_request: bool,
    pub can_open_settings: bool,
}

impl PermissionOutcome {
    pub const fn granted() -> Self {
        Self {
            state: PermissionState::Granted,
            can_request: false,
            can_open_settings: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Finalizing,
    Finalized,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalizationReason {
    UserStop,
    Interruption,
    RouteChange,
    ForegroundExit,
    MediaServicesReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupOutcome {
    Removed,
    NotFound,
    Pending,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSession {
    pub session_id: Option<RecordingSessionId>,
    pub state: RecordingState,
    pub started_at_ms: Option<u64>,
    pub duration_ms: u64,
    pub terminal_reason: Option<FinalizationReason>,
}

impl RecordingSession {
    pub const fn idle() -> Self {
        Self {
            session_id: None,
            state: RecordingState::Idle,
            started_at_ms: None,
            duration_ms: 0,
            terminal_reason: None,
        }
    }

    pub fn recording(session_id: RecordingSessionId) -> Self {
        Self {
            session_id: Some(session_id),
            state: RecordingState::Recording,
            started_at_ms: None,
            duration_ms: 0,
            terminal_reason: None,
        }
    }

    pub fn paused(session_id: RecordingSessionId, duration_ms: u64) -> Self {
        Self {
            session_id: Some(session_id),
            state: RecordingState::Paused,
            started_at_ms: None,
            duration_ms,
            terminal_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizedRecording {
    pub artifact_id: ArtifactId,
    pub session_id: RecordingSessionId,
    pub mime_type: String,
    pub file_extension: String,
    pub duration_ms: u64,
    pub byte_length: u64,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub sha256: String,
    pub finalization_reason: FinalizationReason,
}

impl FinalizedRecording {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        artifact_id: ArtifactId,
        session_id: RecordingSessionId,
        mime_type: impl Into<String>,
        file_extension: impl Into<String>,
        duration_ms: u64,
        byte_length: u64,
        sample_rate_hz: u32,
        channel_count: u16,
        sha256: String,
        finalization_reason: FinalizationReason,
    ) -> Result<Self, RecorderError> {
        let mime_type = mime_type.into();
        let file_extension = file_extension.into();
        let valid_checksum = sha256.len() == 64
            && sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if mime_type != "audio/mp4"
            || file_extension != "m4a"
            || duration_ms == 0
            || byte_length == 0
            || sample_rate_hz == 0
            || channel_count == 0
            || !valid_checksum
        {
            return Err(RecorderError::new(
                RecorderErrorCode::InvalidArtifact,
                Some(session_id),
                false,
            ));
        }
        Ok(Self {
            artifact_id,
            session_id,
            mime_type,
            file_extension,
            duration_ms,
            byte_length,
            sample_rate_hz,
            channel_count,
            sha256,
            finalization_reason,
        })
    }

    #[cfg(test)]
    pub(crate) fn fixture(session_id: RecordingSessionId) -> Self {
        Self::new(
            ArtifactId::new(),
            session_id,
            "audio/mp4",
            "m4a",
            1_000,
            128,
            44_100,
            1,
            "a".repeat(64),
            FinalizationReason::UserStop,
        )
        .expect("valid fixture")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecorderEvent {
    pub event_id: Uuid,
    pub session_id: RecordingSessionId,
    pub sequence: u64,
    pub state: RecordingState,
    pub reason: Option<FinalizationReason>,
    pub recording: Option<FinalizedRecording>,
    pub cleanup: Option<CleanupOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RecorderErrorCode {
    InvalidSessionId,
    ActiveSessionExists,
    InvalidTransition,
    StaleSession,
    PermissionDenied,
    PermissionRestricted,
    PermissionRequestUnavailable,
    StorageUnavailable,
    AudioSessionFailure,
    RecorderFailure,
    FinalizationFailure,
    InvalidArtifact,
    CleanupFailure,
    TerminalConflict,
    UnsupportedPlatform,
    NativeCommandFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}")]
#[serde(rename_all = "camelCase")]
pub struct RecorderError {
    pub code: RecorderErrorCode,
    pub session_id: Option<RecordingSessionId>,
    pub retryable: bool,
    pub cleanup: Option<CleanupOutcome>,
}

impl RecorderError {
    pub const fn new(
        code: RecorderErrorCode,
        session_id: Option<RecordingSessionId>,
        retryable: bool,
    ) -> Self {
        Self {
            code,
            session_id,
            retryable,
            cleanup: None,
        }
    }

    pub const fn with_cleanup(mut self, cleanup: CleanupOutcome) -> Self {
        self.cleanup = Some(cleanup);
        self
    }

    pub const fn public_message(&self) -> &'static str {
        match self.code {
            RecorderErrorCode::InvalidSessionId => "invalid recording session identifier",
            RecorderErrorCode::ActiveSessionExists => "a recording session is already active",
            RecorderErrorCode::InvalidTransition => {
                "recorder action is invalid in the current state"
            }
            RecorderErrorCode::StaleSession => "recording session is no longer active",
            RecorderErrorCode::PermissionDenied => "microphone permission is denied",
            RecorderErrorCode::PermissionRestricted => "microphone permission is restricted",
            RecorderErrorCode::PermissionRequestUnavailable => {
                "microphone permission request is unavailable"
            }
            RecorderErrorCode::StorageUnavailable => "recording storage is unavailable",
            RecorderErrorCode::AudioSessionFailure => "audio session is unavailable",
            RecorderErrorCode::RecorderFailure => "recorder operation failed",
            RecorderErrorCode::FinalizationFailure => "recorder finalization failed",
            RecorderErrorCode::InvalidArtifact => "finalized recording is invalid",
            RecorderErrorCode::CleanupFailure => "recording cleanup failed",
            RecorderErrorCode::TerminalConflict => "recording session already ended differently",
            RecorderErrorCode::UnsupportedPlatform => "recording is unsupported on this platform",
            RecorderErrorCode::NativeCommandFailed => "native recorder command failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalOutcome {
    Finalized(FinalizedRecording),
    Cancelled(CleanupOutcome),
    Failed(RecorderError),
}

#[derive(Debug, Default)]
pub struct RecordingLifecycle {
    current: Option<RecordingSession>,
    terminal: HashMap<RecordingSessionId, TerminalOutcome>,
}

impl RecordingLifecycle {
    pub fn current(&self) -> RecordingSession {
        self.current.clone().unwrap_or_else(RecordingSession::idle)
    }

    pub fn terminal(&self, session_id: &RecordingSessionId) -> Option<&TerminalOutcome> {
        self.terminal.get(session_id)
    }

    pub fn observe(&mut self, session: RecordingSession) {
        self.current = Some(session);
    }

    pub fn begin(
        &mut self,
        session_id: RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        if self
            .current
            .as_ref()
            .is_some_and(|session| !Self::is_terminal(session.state))
        {
            return Err(RecorderError::new(
                RecorderErrorCode::ActiveSessionExists,
                Some(session_id),
                false,
            ));
        }
        let session = RecordingSession::recording(session_id);
        self.current = Some(session.clone());
        Ok(session)
    }

    pub fn pause(
        &mut self,
        session_id: &RecordingSessionId,
        duration_ms: u64,
    ) -> Result<RecordingSession, RecorderError> {
        let current = self.require_session(session_id)?;
        match current.state {
            RecordingState::Paused => Ok(current),
            RecordingState::Recording => {
                let paused = RecordingSession::paused(session_id.clone(), duration_ms);
                self.current = Some(paused.clone());
                Ok(paused)
            }
            _ => Err(Self::invalid_transition(session_id)),
        }
    }

    pub fn resume(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        let current = self.require_session(session_id)?;
        match current.state {
            RecordingState::Recording => Ok(current),
            RecordingState::Paused => {
                let mut recording = current;
                recording.state = RecordingState::Recording;
                self.current = Some(recording.clone());
                Ok(recording)
            }
            _ => Err(Self::invalid_transition(session_id)),
        }
    }

    pub fn begin_finalization(
        &mut self,
        session_id: &RecordingSessionId,
        reason: FinalizationReason,
    ) -> Result<RecordingSession, RecorderError> {
        let mut current = self.require_session(session_id)?;
        if !matches!(
            current.state,
            RecordingState::Recording | RecordingState::Paused
        ) {
            return Err(Self::invalid_transition(session_id));
        }
        current.state = RecordingState::Finalizing;
        current.terminal_reason = Some(reason);
        self.current = Some(current.clone());
        Ok(current)
    }

    pub fn begin_cancellation(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        let mut current = self.require_active_or_paused(session_id)?;
        current.state = RecordingState::Finalizing;
        current.terminal_reason = None;
        self.current = Some(current.clone());
        Ok(current)
    }

    pub fn finalize(&mut self, recording: FinalizedRecording) {
        let session_id = recording.session_id.clone();
        self.current = Some(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: recording.duration_ms,
            terminal_reason: Some(recording.finalization_reason),
        });
        self.terminal
            .insert(session_id, TerminalOutcome::Finalized(recording));
    }

    pub fn cancel(
        &mut self,
        session_id: &RecordingSessionId,
        cleanup: CleanupOutcome,
    ) -> Result<(), RecorderError> {
        self.require_active_or_paused(session_id)?;
        self.current = Some(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Cancelled,
            started_at_ms: None,
            duration_ms: 0,
            terminal_reason: None,
        });
        self.terminal
            .insert(session_id.clone(), TerminalOutcome::Cancelled(cleanup));
        Ok(())
    }

    pub fn complete_cancel(&mut self, session_id: &RecordingSessionId, cleanup: CleanupOutcome) {
        self.current = Some(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Cancelled,
            started_at_ms: None,
            duration_ms: 0,
            terminal_reason: None,
        });
        self.terminal
            .insert(session_id.clone(), TerminalOutcome::Cancelled(cleanup));
    }

    pub fn fail(&mut self, session_id: &RecordingSessionId, error: RecorderError) {
        self.current = Some(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Failed,
            started_at_ms: None,
            duration_ms: 0,
            terminal_reason: None,
        });
        self.terminal
            .insert(session_id.clone(), TerminalOutcome::Failed(error));
    }

    pub fn require_active_or_paused(
        &self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        let current = self.require_session(session_id)?;
        if matches!(
            current.state,
            RecordingState::Recording | RecordingState::Paused
        ) {
            Ok(current)
        } else {
            Err(Self::invalid_transition(session_id))
        }
    }

    fn require_session(
        &self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        match &self.current {
            Some(current) if current.session_id.as_ref() == Some(session_id) => Ok(current.clone()),
            _ => Err(RecorderError::new(
                RecorderErrorCode::StaleSession,
                Some(session_id.clone()),
                false,
            )),
        }
    }

    const fn is_terminal(state: RecordingState) -> bool {
        matches!(
            state,
            RecordingState::Idle
                | RecordingState::Finalized
                | RecordingState::Cancelled
                | RecordingState::Failed
        )
    }

    fn invalid_transition(session_id: &RecordingSessionId) -> RecorderError {
        RecorderError::new(
            RecorderErrorCode::InvalidTransition,
            Some(session_id.clone()),
            false,
        )
    }
}
