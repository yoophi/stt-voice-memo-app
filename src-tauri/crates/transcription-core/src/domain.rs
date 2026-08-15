use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

macro_rules! opaque_string_id {
    ($name:ident, $error:literal) => {
        #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into();
                if value.trim().is_empty()
                    || value.len() > 128
                    || value.chars().any(char::is_control)
                {
                    return Err(DomainError::InvalidValue($error));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.0)
                    .finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TranscriptionOperationId(Uuid);

impl TranscriptionOperationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        let id = Uuid::parse_str(value)
            .map_err(|_| DomainError::InvalidValue("transcription operation id"))?;
        if id.to_string() != value {
            return Err(DomainError::InvalidValue("transcription operation id"));
        }
        Ok(Self(id))
    }
}

impl Default for TranscriptionOperationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TranscriptionOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("TranscriptionOperationId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for TranscriptionOperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

opaque_string_id!(BackendOperationId, "backend operation id");
opaque_string_id!(BackendRequestId, "backend request id");
opaque_string_id!(SourceAudioId, "source audio id");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubmissionFingerprint(String);

impl SubmissionFingerprint {
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DomainError::InvalidValue("submission fingerprint"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn derive(source: &SourceDescriptor, options: &TranscriptionOptions) -> Self {
        let mut digest = Sha256::new();
        digest.update(b"transcription-api:v1\0");
        digest.update(source.id.as_str().as_bytes());
        digest.update(b"\0");
        digest.update(source.sha256.as_bytes());
        digest.update(b"\0");
        digest.update(options.language_hint.as_deref().unwrap_or("-"));
        Self(format!("{:x}", digest.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOptions {
    pub language_hint: Option<String>,
}

impl TranscriptionOptions {
    pub fn new(language_hint: Option<&str>) -> Result<Self, DomainError> {
        let language_hint = language_hint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        if language_hint.as_ref().is_some_and(|value| {
            value.len() > 35
                || value.starts_with('-')
                || value.ends_with('-')
                || value
                    .bytes()
                    .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'-'))
        }) {
            return Err(DomainError::InvalidValue("language hint"));
        }
        Ok(Self { language_hint })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDescriptor {
    pub id: SourceAudioId,
    pub media_type: String,
    pub file_extension: String,
    pub byte_length: u64,
    pub duration_ms: u64,
    pub sha256: String,
}

impl SourceDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SourceAudioId,
        media_type: impl Into<String>,
        file_extension: impl Into<String>,
        byte_length: u64,
        duration_ms: u64,
        sha256: String,
    ) -> Result<Self, DomainError> {
        let media_type = media_type.into();
        let file_extension = file_extension.into().to_ascii_lowercase();
        let format_valid = matches!(
            (media_type.as_str(), file_extension.as_str()),
            ("audio/mp4", "m4a" | "mp4")
                | ("audio/mpeg", "mp3" | "mpeg" | "mpga")
                | ("audio/wav" | "audio/x-wav", "wav")
                | ("audio/webm", "webm")
        );
        let checksum_valid = sha256.len() == 64
            && sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !format_valid
            || !(1..=25_000_000).contains(&byte_length)
            || !(1..=600_000).contains(&duration_ms)
            || !checksum_valid
        {
            return Err(DomainError::InvalidValue("source descriptor"));
        }
        Ok(Self {
            id,
            media_type,
            file_extension,
            byte_length,
            duration_ms,
            sha256,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureCategory {
    Retryable,
    UserActionable,
    Terminal,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub code: String,
    pub category: FailureCategory,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub request_id: Option<BackendRequestId>,
}

impl Failure {
    pub fn new(
        code: impl Into<String>,
        category: FailureCategory,
        retry_after_ms: Option<u64>,
    ) -> Result<Self, DomainError> {
        let code = code.into();
        if code.is_empty()
            || code.len() > 64
            || code
                .bytes()
                .any(|byte| !(byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'))
        {
            return Err(DomainError::InvalidValue("failure code"));
        }
        Ok(Self {
            code,
            category,
            retryable: category == FailureCategory::Retryable,
            retry_after_ms,
            request_id: None,
        })
    }

    pub fn with_request_id(mut self, request_id: BackendRequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn is_authentication_required(&self) -> bool {
        self.code == "AUTHENTICATION_REQUIRED" && self.category == FailureCategory::UserActionable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationPhase {
    Ready,
    WaitingForNetwork,
    WaitingForAuthorization,
    Uploading,
    Queued,
    Processing,
    Completed,
    RetryableFailure,
    Uncertain,
    TerminalFailure,
    Cancelling,
    Cancelled,
    CleanupPending,
}

impl OperationPhase {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::TerminalFailure | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalWinner {
    Completed,
    Cancelled,
    TerminalFailure,
}

impl fmt::Display for TerminalWinner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::TerminalFailure => "terminalFailure",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CleanupDisposition {
    #[default]
    NotScheduled,
    Scheduled {
        delete_by_ms: u64,
    },
    InProgress {
        delete_by_ms: u64,
    },
    FailedRetrying {
        delete_by_ms: u64,
    },
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryMetadata {
    pub earliest_retry_at_ms: u64,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadObservation {
    pub operation_id: TranscriptionOperationId,
    pub attempt: u32,
    pub sequence: u64,
    pub supplied_bytes: u64,
    pub total_bytes: u64,
}

impl UploadObservation {
    pub fn new(
        operation_id: TranscriptionOperationId,
        attempt: u32,
        sequence: u64,
        supplied_bytes: u64,
        total_bytes: u64,
    ) -> Result<Self, DomainError> {
        if attempt == 0 || sequence == 0 || total_bytes == 0 || supplied_bytes > total_bytes {
            return Err(DomainError::InvalidValue("upload observation"));
        }
        Ok(Self {
            operation_id,
            attempt,
            sequence,
            supplied_bytes,
            total_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionOperation {
    id: TranscriptionOperationId,
    source_audio_id: SourceAudioId,
    fingerprint: SubmissionFingerprint,
    options: TranscriptionOptions,
    backend_operation_id: Option<BackendOperationId>,
    phase: OperationPhase,
    attempt: u32,
    progress: Option<UploadObservation>,
    terminal_winner: Option<TerminalWinner>,
    failure: Option<Failure>,
    retry: Option<RetryMetadata>,
    #[serde(default)]
    poll_at_ms: Option<u64>,
    cleanup: CleanupDisposition,
    backend_request_id: Option<BackendRequestId>,
    event_sequence: u64,
    revision: u64,
    #[serde(default)]
    cancel_requested: bool,
}

impl TranscriptionOperation {
    pub fn new(
        id: TranscriptionOperationId,
        source_audio_id: SourceAudioId,
        fingerprint: SubmissionFingerprint,
        options: TranscriptionOptions,
    ) -> Self {
        Self {
            id,
            source_audio_id,
            fingerprint,
            options,
            backend_operation_id: None,
            phase: OperationPhase::Ready,
            attempt: 0,
            progress: None,
            terminal_winner: None,
            failure: None,
            retry: None,
            poll_at_ms: None,
            cleanup: CleanupDisposition::NotScheduled,
            backend_request_id: None,
            event_sequence: 0,
            revision: 0,
            cancel_requested: false,
        }
    }

    pub fn id(&self) -> &TranscriptionOperationId {
        &self.id
    }
    pub fn source_audio_id(&self) -> &SourceAudioId {
        &self.source_audio_id
    }
    pub fn fingerprint(&self) -> &SubmissionFingerprint {
        &self.fingerprint
    }
    pub fn options(&self) -> &TranscriptionOptions {
        &self.options
    }
    pub fn backend_operation_id(&self) -> Option<&BackendOperationId> {
        self.backend_operation_id.as_ref()
    }
    pub fn phase(&self) -> OperationPhase {
        self.phase
    }
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    pub fn progress(&self) -> Option<&UploadObservation> {
        self.progress.as_ref()
    }
    pub fn terminal_winner(&self) -> Option<TerminalWinner> {
        self.terminal_winner
    }
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }
    pub fn retry(&self) -> Option<&RetryMetadata> {
        self.retry.as_ref()
    }
    pub fn poll_at_ms(&self) -> Option<u64> {
        self.poll_at_ms
    }
    pub fn cleanup(&self) -> &CleanupDisposition {
        &self.cleanup
    }
    pub fn backend_request_id(&self) -> Option<&BackendRequestId> {
        self.backend_request_id.as_ref()
    }
    pub fn event_sequence(&self) -> u64 {
        self.event_sequence
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested
    }

    pub fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    pub fn mark_waiting_for_network(&mut self) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        if !matches!(
            self.phase,
            OperationPhase::Ready | OperationPhase::WaitingForNetwork
        ) {
            return Err(DomainError::InvalidTransition);
        }
        self.phase = OperationPhase::WaitingForNetwork;
        Ok(())
    }

    pub fn mark_waiting_for_authorization(&mut self, failure: Failure) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        if failure.category != FailureCategory::UserActionable {
            return Err(DomainError::InvalidValue("authorization failure"));
        }
        self.phase = OperationPhase::WaitingForAuthorization;
        self.failure = Some(failure);
        self.retry = None;
        self.poll_at_ms = None;
        Ok(())
    }

    pub fn begin_upload(&mut self, now_ms: u64) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        if !matches!(
            self.phase,
            OperationPhase::Ready
                | OperationPhase::WaitingForNetwork
                | OperationPhase::WaitingForAuthorization
                | OperationPhase::RetryableFailure
                | OperationPhase::Uncertain
                | OperationPhase::Cancelling
        ) {
            return Err(DomainError::InvalidTransition);
        }
        if self
            .retry
            .as_ref()
            .is_some_and(|retry| now_ms < retry.earliest_retry_at_ms)
        {
            return Err(DomainError::RetryNotReady);
        }
        if self
            .retry
            .as_ref()
            .is_some_and(|retry| self.attempt >= retry.max_attempts)
        {
            return Err(DomainError::RetryExhausted);
        }
        self.attempt = self
            .attempt
            .checked_add(1)
            .ok_or(DomainError::RetryExhausted)?;
        self.phase = OperationPhase::Uploading;
        self.failure = None;
        self.retry = None;
        self.poll_at_ms = None;
        self.progress = None;
        Ok(())
    }

    pub fn observe_progress(
        &mut self,
        observation: UploadObservation,
    ) -> Result<bool, DomainError> {
        if self.terminal_winner.is_some()
            || self.phase != OperationPhase::Uploading
            || observation.operation_id != self.id
            || observation.attempt != self.attempt
            || self.progress.as_ref().is_some_and(|current| {
                observation.sequence <= current.sequence
                    || observation.supplied_bytes < current.supplied_bytes
                    || observation.total_bytes != current.total_bytes
            })
        {
            return Ok(false);
        }
        self.progress = Some(observation);
        Ok(true)
    }

    pub fn observe_backend_active(
        &mut self,
        id: BackendOperationId,
        phase: OperationPhase,
        request_id: Option<BackendRequestId>,
        poll_at_ms: Option<u64>,
    ) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        if !matches!(phase, OperationPhase::Queued | OperationPhase::Processing) {
            return Err(DomainError::InvalidTransition);
        }
        self.bind_backend_id(id)?;
        self.phase = phase;
        self.backend_request_id = request_id;
        self.failure = None;
        self.poll_at_ms = poll_at_ms;
        Ok(())
    }

    pub fn complete(&mut self, backend_id: Option<BackendOperationId>) -> Result<(), DomainError> {
        let backend_id = backend_id.ok_or(DomainError::MissingBackendOperationId)?;
        self.bind_backend_id(backend_id)?;
        self.choose_terminal(TerminalWinner::Completed)?;
        self.phase = OperationPhase::Completed;
        self.failure = None;
        self.retry = None;
        self.poll_at_ms = None;
        Ok(())
    }

    pub fn fail(&mut self, failure: Failure, now_ms: u64) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        self.backend_request_id = failure.request_id.clone();
        self.poll_at_ms = None;
        match failure.category {
            FailureCategory::Retryable => {
                self.phase = OperationPhase::RetryableFailure;
                self.retry = Some(RetryMetadata {
                    earliest_retry_at_ms: now_ms
                        .saturating_add(failure.retry_after_ms.unwrap_or(0)),
                    max_attempts: 5,
                });
            }
            FailureCategory::Uncertain => self.phase = OperationPhase::Uncertain,
            FailureCategory::UserActionable | FailureCategory::Terminal => {
                self.choose_terminal(TerminalWinner::TerminalFailure)?;
                self.phase = OperationPhase::TerminalFailure;
            }
        }
        self.failure = Some(failure);
        Ok(())
    }

    pub fn fail_terminal(&mut self, failure: Failure) -> Result<(), DomainError> {
        if !matches!(
            failure.category,
            FailureCategory::Terminal | FailureCategory::UserActionable
        ) {
            return Err(DomainError::InvalidValue("terminal failure"));
        }
        self.fail(failure, 0)
    }

    pub fn begin_cancel(&mut self) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        self.cancel_requested = true;
        self.phase = OperationPhase::Cancelling;
        Ok(())
    }

    pub fn cancel_local(&mut self) -> Result<(), DomainError> {
        self.cancel_requested = true;
        self.choose_terminal(TerminalWinner::Cancelled)?;
        self.phase = OperationPhase::Cancelled;
        self.cleanup = CleanupDisposition::Completed;
        self.failure = None;
        self.poll_at_ms = None;
        Ok(())
    }

    pub fn confirm_cancel(&mut self, cleanup: CleanupDisposition) -> Result<(), DomainError> {
        self.cancel_requested = true;
        self.choose_terminal(TerminalWinner::Cancelled)?;
        self.cleanup = cleanup;
        self.phase = if matches!(self.cleanup, CleanupDisposition::Completed) {
            OperationPhase::Cancelled
        } else {
            OperationPhase::CleanupPending
        };
        self.failure = None;
        self.poll_at_ms = None;
        Ok(())
    }

    pub fn mark_cleanup_uncertain(&mut self, failure: Failure) -> Result<(), DomainError> {
        if self.terminal_winner == Some(TerminalWinner::Completed) {
            return Err(DomainError::TerminalConflict);
        }
        self.phase = OperationPhase::CleanupPending;
        self.failure = Some(failure);
        Ok(())
    }

    pub fn mark_cancel_reconciliation_needed(
        &mut self,
        failure: Failure,
    ) -> Result<(), DomainError> {
        self.require_no_terminal()?;
        if !self.cancel_requested {
            return Err(DomainError::InvalidTransition);
        }
        self.phase = OperationPhase::Cancelling;
        self.failure = Some(failure);
        Ok(())
    }

    pub fn set_cleanup(&mut self, cleanup: CleanupDisposition) {
        self.cleanup = cleanup;
    }

    pub fn recover_interrupted_upload(&mut self) -> bool {
        if self.phase == OperationPhase::Uploading {
            self.phase = OperationPhase::Uncertain;
            self.failure = Some(
                Failure::new("INTERRUPTED_TRANSFER", FailureCategory::Uncertain, None)
                    .expect("static failure is valid"),
            );
            true
        } else {
            false
        }
    }

    pub fn next_event_sequence(&mut self) -> u64 {
        self.event_sequence += 1;
        self.event_sequence
    }

    fn bind_backend_id(&mut self, id: BackendOperationId) -> Result<(), DomainError> {
        if self
            .backend_operation_id
            .as_ref()
            .is_some_and(|known| known != &id)
        {
            return Err(DomainError::BackendIdentityMismatch);
        }
        self.backend_operation_id = Some(id);
        Ok(())
    }

    fn choose_terminal(&mut self, winner: TerminalWinner) -> Result<(), DomainError> {
        match self.terminal_winner {
            Some(existing) if existing != winner => Err(DomainError::TerminalConflict),
            Some(_) => Ok(()),
            None => {
                self.terminal_winner = Some(winner);
                Ok(())
            }
        }
    }

    fn require_no_terminal(&self) -> Result<(), DomainError> {
        if self.terminal_winner.is_some() {
            Err(DomainError::TerminalConflict)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct FinalTranscript {
    pub operation_id: TranscriptionOperationId,
    pub backend_operation_id: BackendOperationId,
    text: String,
    pub language: Option<String>,
}

impl FinalTranscript {
    pub fn new(
        operation_id: TranscriptionOperationId,
        backend_operation_id: BackendOperationId,
        text: impl Into<String>,
        language: Option<String>,
    ) -> Result<Self, DomainError> {
        let text = text.into().trim().to_owned();
        if text.is_empty() {
            return Err(DomainError::BlankTranscript);
        }
        Ok(Self {
            operation_id,
            backend_operation_id,
            text,
            language,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Debug for FinalTranscript {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FinalTranscript")
            .field("operation_id", &self.operation_id)
            .field("backend_operation_id", &self.backend_operation_id)
            .field("text", &"[REDACTED]")
            .field("language", &self.language)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("invalid {0}")]
    InvalidValue(&'static str),
    #[error("invalid operation transition")]
    InvalidTransition,
    #[error("a different terminal outcome already won")]
    TerminalConflict,
    #[error("retry is not eligible yet")]
    RetryNotReady,
    #[error("retry policy is exhausted")]
    RetryExhausted,
    #[error("backend operation identity is missing")]
    MissingBackendOperationId,
    #[error("backend operation identity changed")]
    BackendIdentityMismatch,
    #[error("completed transcript is blank")]
    BlankTranscript,
}
