use transcription_core::{
    BackendOperationId, CleanupDisposition, DomainError, Failure, FailureCategory, FinalTranscript,
    OperationPhase, SourceAudioId, SourceDescriptor, SubmissionFingerprint, TranscriptionOperation,
    TranscriptionOperationId, TranscriptionOptions, UploadObservation,
};

fn operation() -> TranscriptionOperation {
    TranscriptionOperation::new(
        TranscriptionOperationId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        SourceAudioId::parse("source-1").unwrap(),
        SubmissionFingerprint::parse(&"a".repeat(64)).unwrap(),
        TranscriptionOptions::default(),
    )
}

#[test]
fn final_transcript_is_nonblank_and_redacted_from_debug_output() {
    let operation_id =
        TranscriptionOperationId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let backend_id = BackendOperationId::parse("backend-1").unwrap();
    assert!(FinalTranscript::new(operation_id.clone(), backend_id.clone(), "  ", None).is_err());
    let transcript =
        FinalTranscript::new(operation_id, backend_id, "sensitive words", None).unwrap();
    assert_eq!(transcript.text(), "sensitive words");
    assert!(!format!("{transcript:?}").contains("sensitive words"));
}

#[test]
fn identifiers_and_source_descriptors_reject_malformed_values() {
    assert!(TranscriptionOperationId::parse("NOT-A-UUID").is_err());
    assert!(BackendOperationId::parse(" ").is_err());
    assert!(SourceAudioId::parse("").is_err());
    assert!(SubmissionFingerprint::parse(&"A".repeat(64)).is_err());
    assert!(
        SourceDescriptor::new(
            SourceAudioId::parse("source-1").unwrap(),
            "audio/mp4",
            "m4a",
            0,
            1_000,
            "b".repeat(64),
        )
        .is_err()
    );
}

#[test]
fn upload_progress_is_attempt_scoped_monotonic_and_terminal_safe() {
    let mut aggregate = operation();
    aggregate.begin_upload(10).unwrap();
    assert!(
        aggregate
            .observe_progress(
                UploadObservation::new(aggregate.id().clone(), 1, 1, 50, 100).unwrap()
            )
            .unwrap()
    );
    assert!(
        !aggregate
            .observe_progress(
                UploadObservation::new(aggregate.id().clone(), 1, 1, 60, 100).unwrap()
            )
            .unwrap()
    );
    assert!(
        !aggregate
            .observe_progress(
                UploadObservation::new(aggregate.id().clone(), 2, 2, 70, 100).unwrap()
            )
            .unwrap()
    );
    assert_eq!(aggregate.progress().unwrap().supplied_bytes, 50);

    aggregate.complete(None).unwrap_err();
}

#[test]
fn first_terminal_winner_is_immutable_and_cleanup_is_orthogonal() {
    let mut aggregate = operation();
    aggregate.begin_upload(10).unwrap();
    aggregate
        .fail_terminal(Failure::new("MALFORMED_RESPONSE", FailureCategory::Terminal, None).unwrap())
        .unwrap();
    assert_eq!(aggregate.phase(), OperationPhase::TerminalFailure);
    assert!(matches!(
        aggregate.cancel_local(),
        Err(DomainError::TerminalConflict)
    ));
    aggregate.set_cleanup(CleanupDisposition::FailedRetrying { delete_by_ms: 500 });
    assert_eq!(aggregate.phase(), OperationPhase::TerminalFailure);
    assert_eq!(
        aggregate.terminal_winner().unwrap().to_string(),
        "terminalFailure"
    );

    for attempt in 1..=5 {
        aggregate.begin_terminal_cleanup_attempt().unwrap();
        aggregate
            .record_terminal_cleanup_failure(
                Failure::new("BACKEND_UNAVAILABLE", FailureCategory::Retryable, Some(0)).unwrap(),
                100,
            )
            .unwrap();
        assert_eq!(aggregate.cleanup_attempts(), attempt);
    }
    assert!(!aggregate.terminal_cleanup_retry_ready(100));
    assert!(matches!(
        aggregate.begin_terminal_cleanup_attempt(),
        Err(DomainError::RetryExhausted)
    ));

    let mut non_retryable = operation();
    non_retryable.begin_upload(10).unwrap();
    non_retryable
        .fail_terminal(Failure::new("MALFORMED_RESPONSE", FailureCategory::Terminal, None).unwrap())
        .unwrap();
    non_retryable.set_cleanup(CleanupDisposition::FailedRetrying { delete_by_ms: 500 });
    non_retryable.begin_terminal_cleanup_attempt().unwrap();
    non_retryable
        .record_terminal_cleanup_failure(
            Failure::new("INVALID_DELETE_RESPONSE", FailureCategory::Terminal, None).unwrap(),
            100,
        )
        .unwrap();
    assert!(non_retryable.retry().is_none());
    assert!(!non_retryable.terminal_cleanup_retry_ready(100));
}
