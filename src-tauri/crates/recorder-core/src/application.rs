use crate::{
    CleanupOutcome, FinalizationReason, FinalizedRecording, PermissionOutcome, RecorderError,
    RecorderErrorCode, RecorderPort, RecordingLifecycle, RecordingSession, RecordingSessionId,
    RecordingState, TerminalOutcome,
};

pub struct RecorderService<P> {
    port: P,
    lifecycle: RecordingLifecycle,
}

impl<P: RecorderPort> RecorderService<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            lifecycle: RecordingLifecycle::default(),
        }
    }

    pub fn port(&self) -> &P {
        &self.port
    }

    pub fn permission_status(&mut self) -> Result<PermissionOutcome, RecorderError> {
        self.port.permission_status()
    }

    pub fn request_permission(&mut self) -> Result<PermissionOutcome, RecorderError> {
        self.port.request_permission()
    }

    pub fn status(
        &mut self,
        session_id: Option<&RecordingSessionId>,
    ) -> Result<RecordingSession, RecorderError> {
        let session = self.port.status(session_id)?;
        self.lifecycle.observe(session.clone());
        Ok(session)
    }

    pub fn start(
        &mut self,
        session_id: RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        if self.lifecycle.begin(session_id.clone()).is_err() {
            let native = self.port.status(None)?;
            self.lifecycle.observe(native);
            self.lifecycle.begin(session_id.clone())?;
        }
        match self.port.start(&session_id) {
            Ok(session) => {
                self.lifecycle.observe(session.clone());
                Ok(session)
            }
            Err(error) => {
                self.lifecycle.fail(&session_id, error.clone());
                Err(error)
            }
        }
    }

    pub fn pause(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        let current = self.lifecycle.require_active_or_paused(session_id)?;
        if current.state == RecordingState::Paused {
            return Ok(current);
        }
        let session = self.port.pause(session_id)?;
        self.lifecycle.pause(session_id, session.duration_ms)
    }

    pub fn resume(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError> {
        let current = self.lifecycle.require_active_or_paused(session_id)?;
        if current.state == RecordingState::Recording {
            return Ok(current);
        }
        self.port.resume(session_id)?;
        self.lifecycle.resume(session_id)
    }

    pub fn stop(
        &mut self,
        session_id: &RecordingSessionId,
        reason: FinalizationReason,
    ) -> Result<FinalizedRecording, RecorderError> {
        if let Some(terminal) = self.lifecycle.terminal(session_id) {
            return match terminal {
                TerminalOutcome::Finalized(recording) => Ok(recording.clone()),
                TerminalOutcome::Failed(error) => Err(error.clone()),
                TerminalOutcome::Cancelled(_) => Err(Self::terminal_conflict(session_id)),
            };
        }
        let observed = self.lifecycle.current();
        if observed.session_id.as_ref() == Some(session_id)
            && matches!(
                observed.state,
                RecordingState::Finalized | RecordingState::Cancelled | RecordingState::Failed
            )
        {
            return match self.port.stop(session_id, reason) {
                Ok(recording) => {
                    self.lifecycle.finalize(recording.clone());
                    Ok(recording)
                }
                Err(error) => Err(error),
            };
        }
        self.lifecycle.begin_finalization(session_id, reason)?;
        match self.port.stop(session_id, reason) {
            Ok(recording) => {
                self.lifecycle.finalize(recording.clone());
                Ok(recording)
            }
            Err(error) => {
                self.lifecycle.fail(session_id, error.clone());
                Err(error)
            }
        }
    }

    pub fn cancel(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<CleanupOutcome, RecorderError> {
        if let Some(terminal) = self.lifecycle.terminal(session_id).cloned() {
            return match terminal {
                TerminalOutcome::Cancelled(cleanup) => Ok(cleanup),
                TerminalOutcome::Failed(error) if Self::cleanup_retryable(&error) => {
                    self.retry_cleanup(session_id)
                }
                TerminalOutcome::Failed(error) => Err(error),
                TerminalOutcome::Finalized(_) => Err(Self::terminal_conflict(session_id)),
            };
        }
        self.lifecycle.require_active_or_paused(session_id)?;
        let result = self.port.cancel(session_id);
        self.apply_cancel_result(session_id, result)
    }

    fn retry_cleanup(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<CleanupOutcome, RecorderError> {
        let result = self.port.cancel(session_id);
        self.apply_cancel_result(session_id, result)
    }

    fn apply_cancel_result(
        &mut self,
        session_id: &RecordingSessionId,
        result: Result<CleanupOutcome, RecorderError>,
    ) -> Result<CleanupOutcome, RecorderError> {
        match result {
            Ok(cleanup @ (CleanupOutcome::Removed | CleanupOutcome::NotFound)) => {
                self.lifecycle.complete_cancel(session_id, cleanup);
                Ok(cleanup)
            }
            Ok(cleanup @ (CleanupOutcome::Pending | CleanupOutcome::Failed)) => {
                let error = RecorderError::new(
                    RecorderErrorCode::CleanupFailure,
                    Some(session_id.clone()),
                    true,
                )
                .with_cleanup(cleanup);
                self.lifecycle.fail(session_id, error.clone());
                Err(error)
            }
            Err(error) => {
                self.lifecycle.fail(session_id, error.clone());
                Err(error)
            }
        }
    }

    fn cleanup_retryable(error: &RecorderError) -> bool {
        error.code == RecorderErrorCode::CleanupFailure
            && error.retryable
            && matches!(
                error.cleanup,
                Some(CleanupOutcome::Pending | CleanupOutcome::Failed)
            )
    }

    fn terminal_conflict(session_id: &RecordingSessionId) -> RecorderError {
        RecorderError::new(
            RecorderErrorCode::TerminalConflict,
            Some(session_id.clone()),
            false,
        )
    }
}
