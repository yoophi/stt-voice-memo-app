pub mod application;
pub mod domain;
pub mod ports;

pub use application::RecorderService;
pub use domain::*;
pub use ports::RecorderPort;

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[derive(Default)]
    struct FakeRecorder {
        calls: Vec<&'static str>,
        status_results: VecDeque<RecordingSession>,
        start_results: VecDeque<Result<RecordingSession, RecorderError>>,
        stop_results: VecDeque<Result<FinalizedRecording, RecorderError>>,
        cleanup_results: VecDeque<Result<CleanupOutcome, RecorderError>>,
        cleanup_result: Option<Result<CleanupOutcome, RecorderError>>,
    }

    impl RecorderPort for FakeRecorder {
        fn permission_status(&mut self) -> Result<PermissionOutcome, RecorderError> {
            self.calls.push("permission_status");
            Ok(PermissionOutcome::granted())
        }

        fn request_permission(&mut self) -> Result<PermissionOutcome, RecorderError> {
            self.calls.push("request_permission");
            Ok(PermissionOutcome::granted())
        }

        fn status(
            &mut self,
            session_id: Option<&RecordingSessionId>,
        ) -> Result<RecordingSession, RecorderError> {
            self.calls.push("status");
            if let Some(session) = self.status_results.pop_front() {
                return Ok(session);
            }
            Ok(match session_id {
                Some(id) => RecordingSession::recording(id.clone()),
                None => RecordingSession::idle(),
            })
        }

        fn start(
            &mut self,
            session_id: &RecordingSessionId,
        ) -> Result<RecordingSession, RecorderError> {
            self.calls.push("start");
            self.start_results
                .pop_front()
                .unwrap_or_else(|| Ok(RecordingSession::recording(session_id.clone())))
        }

        fn pause(
            &mut self,
            session_id: &RecordingSessionId,
        ) -> Result<RecordingSession, RecorderError> {
            self.calls.push("pause");
            Ok(RecordingSession::paused(session_id.clone(), 500))
        }

        fn resume(
            &mut self,
            session_id: &RecordingSessionId,
        ) -> Result<RecordingSession, RecorderError> {
            self.calls.push("resume");
            Ok(RecordingSession::recording(session_id.clone()))
        }

        fn stop(
            &mut self,
            session_id: &RecordingSessionId,
            _reason: FinalizationReason,
        ) -> Result<FinalizedRecording, RecorderError> {
            self.calls.push("stop");
            self.stop_results
                .pop_front()
                .unwrap_or_else(|| Ok(FinalizedRecording::fixture(session_id.clone())))
        }

        fn cancel(
            &mut self,
            _session_id: &RecordingSessionId,
        ) -> Result<CleanupOutcome, RecorderError> {
            self.calls.push("cancel");
            self.cleanup_results
                .pop_front()
                .or_else(|| self.cleanup_result.clone())
                .clone()
                .unwrap_or(Ok(CleanupOutcome::Removed))
        }
    }

    fn id(value: &str) -> RecordingSessionId {
        RecordingSessionId::parse(value).expect("valid UUID")
    }

    #[test]
    fn session_id_requires_canonical_uuid() {
        assert!(RecordingSessionId::parse("not-a-uuid").is_err());
        assert!(RecordingSessionId::parse("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(RecordingSessionId::parse("550E8400-E29B-41D4-A716-446655440000").is_err());
    }

    #[test]
    fn lifecycle_rejects_second_active_session_and_invalid_transitions() {
        let first = id("550e8400-e29b-41d4-a716-446655440000");
        let second = id("a3bb189e-8bf9-3888-9912-ace4e6543002");
        let mut lifecycle = RecordingLifecycle::default();

        lifecycle.begin(first.clone()).unwrap();
        assert_eq!(
            lifecycle.begin(second).unwrap_err().code,
            RecorderErrorCode::ActiveSessionExists
        );
        lifecycle.pause(&first, 125).unwrap();
        lifecycle.pause(&first, 125).unwrap();
        lifecycle.resume(&first).unwrap();
        assert_eq!(
            lifecycle.resume(&first).unwrap().state,
            RecordingState::Recording
        );
    }

    #[test]
    fn finalized_descriptor_rejects_missing_or_inconsistent_metadata() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        assert!(
            FinalizedRecording::new(
                ArtifactId::new(),
                session_id.clone(),
                "audio/mp4",
                "m4a",
                0,
                10,
                44_100,
                1,
                "a".repeat(64),
                FinalizationReason::UserStop,
            )
            .is_err()
        );
        assert!(
            FinalizedRecording::new(
                ArtifactId::new(),
                session_id,
                "audio/wav",
                "wav",
                500,
                10,
                44_100,
                1,
                "a".repeat(64),
                FinalizationReason::UserStop,
            )
            .is_err()
        );
    }

    #[test]
    fn service_calls_port_once_for_repeated_pause_resume_and_stop() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.status_results.extend([
            RecordingSession::recording(session_id.clone()),
            RecordingSession::paused(session_id.clone(), 500),
            RecordingSession::paused(session_id.clone(), 500),
            RecordingSession::recording(session_id.clone()),
        ]);
        let mut service = RecorderService::new(fake);

        service.start(session_id.clone()).unwrap();
        service.pause(&session_id).unwrap();
        service.pause(&session_id).unwrap();
        service.resume(&session_id).unwrap();
        service.resume(&session_id).unwrap();
        let first = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap();
        let second = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(
            service.port().calls,
            vec![
                "start", "status", "pause", "status", "status", "resume", "status", "stop"
            ]
        );
    }

    #[test]
    fn resume_reconciles_native_interruption_before_using_cached_recording_state() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.status_results.push_back(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: 750,
            terminal_reason: Some(FinalizationReason::Interruption),
        });
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        let error = service.resume(&session_id).unwrap_err();

        assert_eq!(error.code, RecorderErrorCode::InvalidTransition);
        assert_eq!(service.port().calls, vec!["start", "status"]);
    }

    #[test]
    fn cancel_retries_cleanup_reported_by_a_failed_start() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.start_results.push_back(Err(RecorderError::new(
            RecorderErrorCode::RecorderFailure,
            Some(session_id.clone()),
            true,
        )
        .with_cleanup(CleanupOutcome::Pending)));
        let mut service = RecorderService::new(fake);

        let start_error = service.start(session_id.clone()).unwrap_err();
        let cleanup = service.cancel(&session_id).unwrap();

        assert_eq!(start_error.code, RecorderErrorCode::RecorderFailure);
        assert_eq!(start_error.cleanup, Some(CleanupOutcome::Pending));
        assert_eq!(cleanup, CleanupOutcome::Removed);
        assert_eq!(service.port().calls, vec!["start", "cancel"]);
    }

    #[test]
    fn start_reconciles_a_native_terminal_state_before_replacing_stale_rust_state() {
        let first = id("550e8400-e29b-41d4-a716-446655440000");
        let second = id("a3bb189e-8bf9-3888-9912-ace4e6543002");
        let mut fake = FakeRecorder::default();
        fake.status_results.push_back(RecordingSession {
            session_id: Some(first.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: 750,
            terminal_reason: Some(FinalizationReason::Interruption),
        });
        let mut service = RecorderService::new(fake);

        service.start(first).unwrap();
        let replacement = service.start(second.clone()).unwrap();

        assert_eq!(replacement.session_id, Some(second));
        assert_eq!(service.port().calls, vec!["start", "status", "start"]);
    }

    #[test]
    fn status_always_refreshes_the_authoritative_native_snapshot() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.status_results.push_back(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: 750,
            terminal_reason: Some(FinalizationReason::ForegroundExit),
        });
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        let status = service.status(Some(&session_id)).unwrap();

        assert_eq!(status.state, RecordingState::Finalized);
        assert_eq!(
            status.terminal_reason,
            Some(FinalizationReason::ForegroundExit)
        );
        assert_eq!(service.port().calls, vec!["start", "status"]);
    }

    #[test]
    fn repeated_stop_resolves_native_result_after_status_observes_system_finalization() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.status_results.push_back(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: 750,
            terminal_reason: Some(FinalizationReason::Interruption),
        });
        let mut finalized = FinalizedRecording::fixture(session_id.clone());
        finalized.finalization_reason = FinalizationReason::Interruption;
        fake.stop_results.push_back(Ok(finalized));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();
        service.status(Some(&session_id)).unwrap();

        let recording = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap();

        assert_eq!(recording.session_id, session_id);
        assert_eq!(service.port().calls, vec!["start", "status", "stop"]);
    }

    #[test]
    fn stale_cancel_after_native_finalization_preserves_the_finalized_winner() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.status_results.push_back(RecordingSession {
            session_id: Some(session_id.clone()),
            state: RecordingState::Finalized,
            started_at_ms: None,
            duration_ms: 750,
            terminal_reason: Some(FinalizationReason::Interruption),
        });
        fake.cleanup_results.push_back(Err(RecorderError::new(
            RecorderErrorCode::TerminalConflict,
            Some(session_id.clone()),
            false,
        )));
        fake.stop_results
            .push_back(Ok(FinalizedRecording::fixture(session_id.clone())));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        let cancel_error = service.cancel(&session_id).unwrap_err();
        let recording = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap();

        assert_eq!(cancel_error.code, RecorderErrorCode::TerminalConflict);
        assert_eq!(recording.session_id, session_id);
        assert_eq!(service.port().calls, vec!["start", "status", "stop"]);
    }

    #[test]
    fn stop_retries_retryable_audio_session_cleanup_without_changing_the_winner() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let audio_session_error = RecorderError::new(
            RecorderErrorCode::AudioSessionFailure,
            Some(session_id.clone()),
            true,
        );
        let mut fake = FakeRecorder::default();
        fake.stop_results
            .push_back(Err(audio_session_error.clone()));
        let mut finalized = FinalizedRecording::fixture(session_id.clone());
        finalized.finalization_reason = FinalizationReason::Interruption;
        fake.stop_results.push_back(Ok(finalized));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        assert_eq!(
            service
                .stop(&session_id, FinalizationReason::Interruption)
                .unwrap_err(),
            audio_session_error
        );
        assert_eq!(
            service.cancel(&session_id).unwrap_err().code,
            RecorderErrorCode::TerminalConflict
        );
        let finalized = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap();

        assert_eq!(
            finalized.finalization_reason,
            FinalizationReason::Interruption
        );
        assert_eq!(service.port().calls, vec!["start", "stop", "stop"]);
    }

    #[test]
    fn cancel_retries_retryable_audio_session_cleanup_without_changing_the_winner() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let audio_session_error = RecorderError::new(
            RecorderErrorCode::AudioSessionFailure,
            Some(session_id.clone()),
            true,
        );
        let mut fake = FakeRecorder::default();
        fake.cleanup_results
            .push_back(Err(audio_session_error.clone()));
        fake.cleanup_results.push_back(Ok(CleanupOutcome::Removed));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        assert_eq!(
            service.cancel(&session_id).unwrap_err(),
            audio_session_error
        );
        assert_eq!(
            service
                .stop(&session_id, FinalizationReason::UserStop)
                .unwrap_err()
                .code,
            RecorderErrorCode::TerminalConflict
        );
        assert_eq!(
            service.cancel(&session_id).unwrap(),
            CleanupOutcome::Removed
        );

        assert_eq!(
            service.port().calls,
            vec!["start", "status", "cancel", "cancel"]
        );
    }

    #[test]
    fn service_surfaces_permission_and_port_failures_without_native_details() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.stop_results.push_back(Err(RecorderError::new(
            RecorderErrorCode::FinalizationFailure,
            Some(session_id.clone()),
            false,
        )));
        let mut service = RecorderService::new(fake);

        assert_eq!(
            service.permission_status().unwrap().state,
            PermissionState::Granted
        );
        service.start(session_id.clone()).unwrap();
        let error = service
            .stop(&session_id, FinalizationReason::UserStop)
            .unwrap_err();
        assert_eq!(error.code, RecorderErrorCode::FinalizationFailure);
        assert_eq!(error.public_message(), "recorder finalization failed");
    }

    #[test]
    fn cancel_is_idempotent_for_successful_cleanup() {
        for cleanup in [CleanupOutcome::Removed, CleanupOutcome::NotFound] {
            let session_id = id("550e8400-e29b-41d4-a716-446655440000");
            let fake = FakeRecorder {
                cleanup_result: Some(Ok(cleanup)),
                ..FakeRecorder::default()
            };
            let mut service = RecorderService::new(fake);
            service.start(session_id.clone()).unwrap();

            assert_eq!(service.cancel(&session_id).unwrap(), cleanup);
            assert_eq!(service.cancel(&session_id).unwrap(), cleanup);
            assert_eq!(service.port().calls, vec!["start", "status", "cancel"]);
        }
    }

    #[test]
    fn pending_cleanup_is_retried_by_a_repeated_cancel() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut fake = FakeRecorder::default();
        fake.cleanup_results.push_back(Ok(CleanupOutcome::Pending));
        fake.cleanup_results.push_back(Ok(CleanupOutcome::Removed));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        let error = service.cancel(&session_id).unwrap_err();
        assert_eq!(error.code, RecorderErrorCode::CleanupFailure);
        assert!(error.retryable);
        assert_eq!(error.cleanup, Some(CleanupOutcome::Pending));
        assert_eq!(
            service.cancel(&session_id).unwrap(),
            CleanupOutcome::Removed
        );
        assert_eq!(
            service.port().calls,
            vec!["start", "status", "cancel", "cancel"]
        );
    }

    #[test]
    fn cleanup_error_is_retried_by_a_repeated_cancel() {
        let session_id = id("550e8400-e29b-41d4-a716-446655440000");
        let cleanup_error = RecorderError::new(
            RecorderErrorCode::CleanupFailure,
            Some(session_id.clone()),
            true,
        )
        .with_cleanup(CleanupOutcome::Failed);
        let mut fake = FakeRecorder::default();
        fake.cleanup_results.push_back(Err(cleanup_error.clone()));
        fake.cleanup_results.push_back(Ok(CleanupOutcome::NotFound));
        let mut service = RecorderService::new(fake);
        service.start(session_id.clone()).unwrap();

        assert_eq!(service.cancel(&session_id).unwrap_err(), cleanup_error);
        assert_eq!(
            service.cancel(&session_id).unwrap(),
            CleanupOutcome::NotFound
        );
        assert_eq!(
            service.port().calls,
            vec!["start", "status", "cancel", "cancel"]
        );
    }

    #[test]
    fn stop_and_cancel_conflicts_preserve_the_first_terminal_result() {
        let cancelled_id = id("550e8400-e29b-41d4-a716-446655440000");
        let mut cancelled = RecorderService::new(FakeRecorder::default());
        cancelled.start(cancelled_id.clone()).unwrap();
        cancelled.cancel(&cancelled_id).unwrap();
        assert_eq!(
            cancelled
                .stop(&cancelled_id, FinalizationReason::UserStop)
                .unwrap_err()
                .code,
            RecorderErrorCode::TerminalConflict
        );

        let finalized_id = id("a3bb189e-8bf9-3888-9912-ace4e6543002");
        let mut finalized = RecorderService::new(FakeRecorder::default());
        finalized.start(finalized_id.clone()).unwrap();
        finalized
            .stop(&finalized_id, FinalizationReason::UserStop)
            .unwrap();
        assert_eq!(
            finalized.cancel(&finalized_id).unwrap_err().code,
            RecorderErrorCode::TerminalConflict
        );
    }
}
