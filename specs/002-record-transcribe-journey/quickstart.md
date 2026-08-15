# Validation Quickstart: Record and Transcribe Memo Journey Contract

## Purpose

This feature is complete when its specification and contracts are internally
consistent and give Issues #3 through #7 an unambiguous implementation target.
It does not add runtime recording behavior, so the device scenarios below are
acceptance procedures for those follow-up implementations, not evidence that
recording already works.

## Contract review

1. Confirm every state in `contracts/journey-state-machine.md` has a user-visible
   meaning, valid actions, and a recovery/terminal outcome.
2. Trace `RecordingSessionId`, `SourceAudioId`,
   `TranscriptionOperationId`, and `MemoId` through `data-model.md` and verify
   repeated actions never create a second logical operation.
3. Trace original audio from device creation through upload, success, failure,
   cancellation, optional retention, and deletion.
4. Confirm `contracts/recorder-port.md` produces the same product outcomes for
   iOS and Android while allowing different native mechanics.
5. Confirm `contracts/transcription-boundary.md` contains no client OpenAI key,
   client model selection, or assumption of provider idempotency.
6. Confirm production code and microphone permissions remain unchanged until
   their owning follow-up issues are implemented.

## Requirement traceability

| Acceptance area               | Primary requirements                          | Contract evidence                               |
| ----------------------------- | --------------------------------------------- | ----------------------------------------------- |
| Primary record-to-save flow   | FR-001–FR-006                                 | Journey state machine; data model               |
| Failure and relaunch recovery | FR-007, FR-010–FR-012                         | Journey relaunch/error rules; recorder port     |
| Security and privacy          | FR-008–FR-009, FR-013–FR-015, PDL-001–PDL-004 | Transcription boundary; lifecycle tables        |
| Mobile lifecycle              | MLR-001–MLR-005                               | Recorder cross-platform behavior; device matrix |
| Scope exclusions              | FR-016                                        | Spec assumptions and deferred ownership         |

## Future physical-device matrix

Each applicable follow-up issue records device model, OS version, build commit,
date, expected result, actual result, and artifact/log reference without recording
or transcript content.

| Scenario                                          | iOS physical device | Android physical device | Required result                                        |
| ------------------------------------------------- | ------------------- | ----------------------- | ------------------------------------------------------ |
| First permission grant and normal two-minute memo | Required            | Required                | One editable draft and one saved memo                  |
| Denial, repeat attempt, settings recovery         | Required            | Required                | No prompt loop; recording only after grant             |
| Foreground recording then Home/app switch         | Required            | Required                | Capture ends/finalizes; no hidden continuation         |
| Incoming call/system assistant/alarm              | Required            | Required                | Partial finalized or explicit loss; no auto-resume     |
| Wired/Bluetooth input route change                | Required            | Required                | Clear interruption outcome and valid source metadata   |
| Competing microphone capture                      | Required            | Required                | No silent success with unusable audio                  |
| Offline stop, relaunch, reconnect, retry          | Required            | Required                | Same operation ID; source preserved; one result        |
| Slow upload and cancellation                      | Required            | Required                | Progress/cancel visible; late result ignored           |
| Force termination during capture                  | Required            | Required                | Best-effort recovery or explicit unrecoverable result  |
| Force termination during processing               | Required            | Required                | Durable status resolves same operation after relaunch  |
| Repeated stop/transcribe/retry/save taps          | Required            | Required                | One source, operation, and memo                        |
| Default deletion after save                       | Required            | Required                | Local source gone; backend cleanup scheduled/completed |
| Explicit retain then memo deletion                | Required            | Required                | Audio retained only until user deletion                |
| Low storage/encoder failure                       | Required            | Required                | No false saved state; actionable recovery              |

Baseline coverage includes iOS 15+ and Android API 24+ behavior; current OS
coverage is added before release. Simulator/emulator runs are supplementary.

## Follow-up handoff

- Issue #3 consumes `transcription-boundary.md` and supplies the production wire
  schema, auth, hard limits, region/model policy, and backend cleanup proof.
- Issue #4 consumes `recorder-port.md` for the iOS adapter. Android adapter work
  must be explicitly scheduled against the same contract.
- Issue #5 consumes `data-model.md` and `journey-state-machine.md` for Rust
  domain/port/use-case tests.
- Issue #6 validates stable IDs, durable recovery, event reconciliation, and
  idempotency end to end.
- Issue #7 implements the accessible touch UI and audio-retention controls.
