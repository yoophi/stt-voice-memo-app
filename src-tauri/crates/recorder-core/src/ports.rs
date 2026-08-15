use crate::{
    CleanupOutcome, FinalizationReason, FinalizedRecording, PermissionOutcome, RecorderError,
    RecordingSession, RecordingSessionId,
};

pub trait RecorderPort {
    fn permission_status(&mut self) -> Result<PermissionOutcome, RecorderError>;
    fn request_permission(&mut self) -> Result<PermissionOutcome, RecorderError>;
    fn status(
        &mut self,
        session_id: Option<&RecordingSessionId>,
    ) -> Result<RecordingSession, RecorderError>;
    fn start(&mut self, session_id: &RecordingSessionId)
    -> Result<RecordingSession, RecorderError>;
    fn pause(&mut self, session_id: &RecordingSessionId)
    -> Result<RecordingSession, RecorderError>;
    fn resume(
        &mut self,
        session_id: &RecordingSessionId,
    ) -> Result<RecordingSession, RecorderError>;
    fn stop(
        &mut self,
        session_id: &RecordingSessionId,
        reason: FinalizationReason,
    ) -> Result<FinalizedRecording, RecorderError>;
    fn cancel(&mut self, session_id: &RecordingSessionId) -> Result<CleanupOutcome, RecorderError>;
}
