# Implementation Readiness: Record and Transcribe Memo Journey

**Purpose**: Prove that Issue #2 gives downstream implementation issues one
consistent, testable contract without claiming that runtime recording exists.

**Created**: 2026-08-15

**Feature**: [spec.md](../spec.md)

## User Story 1: Primary Journey

| Evidence ID          | Observable contract                                                                                                                               | Source                                                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `US1-RECORD`         | One visible user action creates at most one foreground recording session after permission is granted.                                             | FR-001–FR-003; `journey-state-machine.md` start/stop events           |
| `US1-FINAL`          | A stopped capture becomes one locally identifiable original source before any explicit submission. Only a non-blank final result becomes a draft. | FR-003–FR-005; `data-model.md` SourceAudio and TranscriptionOperation |
| `US1-EDIT`           | The returned final text is component-local editable draft state until save and cannot auto-save empty text.                                       | FR-005; `data-model.md` TranscriptDraft                               |
| `US1-SAVE`           | Repeated save resolves to one `MemoId` containing the user's edited text.                                                                         | FR-006, FR-012; `data-model.md` Memo                                  |
| `US1-DELETE-DEFAULT` | Successful save deletes local source audio unless the user explicitly selected retention.                                                         | FR-008; privacy lifecycle table                                       |

### US1 independent review

- [x] `RecordingSessionId`, `SourceAudioId`, `TranscriptionOperationId`, and
      `MemoId` have distinct lifetimes and no content-bearing identifiers.
- [x] Canonical state names are identical in the data model and journey contract.
- [x] Partial provider output cannot become an editable or saved memo.
- [x] Production source, capabilities, and the foundation shell remain unchanged.

## User Story 2: Recovery Journey

| Evidence ID        | Failure/recovery contract                                                                                                                              | Platform / source                                         |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| `REC-PERMISSION`   | Denial or restriction prevents capture, does not loop prompts, and offers settings/help. A new attempt rechecks permission.                            | iOS and Android; MLR-001; recorder port                   |
| `REC-INTERRUPTION` | Call, assistant, route change, encoder error, or capture contention stops and best-effort finalizes; never auto-resumes.                               | iOS audio session / Android recorder observation; MLR-003 |
| `REC-BACKGROUND`   | Actual iOS scene background or Android non-visible `ON_STOP` ends foreground capture; no background capability/service is enabled.                     | MLR-002; research Decision 2                              |
| `REC-OFFLINE`      | Explicit submission while offline preserves source and stable operation metadata in `queued_offline`; retry/cancel remain available.                   | FR-007, FR-010; journey contract                          |
| `REC-UNCERTAIN`    | Timeout or unknown provider outcome resolves backend state before another attempt and reuses the operation identity.                                   | FR-011–FR-012; transcription boundary                     |
| `REC-DUPLICATE`    | Repeated start, stop, transcribe, retry, and save return the existing session/source/operation/memo result.                                            | FR-002, FR-012; duplicate action rules                    |
| `REC-CANCEL-LATE`  | Cancellation invalidates queued work; a late final result is ignored and cannot recreate deleted content.                                              | FR-009; PDL-002; journey contract                         |
| `REC-TERMINATION`  | Relaunch recovers durable finalized audio/operation metadata or reports an explicit unrecoverable capture; recovered audio is never silently uploaded. | FR-010; recorder relaunch rules                           |

### US2 platform ownership

- [x] Swift/iOS Issue #4 owns system permission, audio-session interruption,
      route-change, and scene-background mechanics behind the common recorder port.
- [x] Android must receive an explicit native-adapter implementation issue using
      the same port contract; this dependency risk does not weaken the defined
      Android acceptance contract.
- [x] Issue #5 owns pure Rust transition and port contract tests without native,
      filesystem, HTTP, Tauri, or provider dependencies.
- [x] Issue #6 owns durable queue/status reconciliation, late-result rejection,
      and end-to-end stable identity tests.
- [x] `quickstart.md` contains physical-device scenarios for every listed
      recovery family; Issue #2 records procedures, not false execution results.

## User Story 3: Privacy and Retention Journey

| Evidence ID        | Lifecycle/security contract                                                                                                                      | Source / owner                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| `PRIV-LOCAL`       | Original audio stays app-private through recoverable work, is deleted after successful save by default, and is retained only by explicit choice. | FR-007–FR-009; PDL-001–PDL-004; Issues #5–#7                   |
| `PRIV-BACKEND`     | App-controlled upload copies are terminally cleaned within 24 hours; cancellation schedules cleanup and ignores late output.                     | PDL-002–PDL-003; Issue #3 implementation, #6 integration proof |
| `PRIV-PROVIDER`    | Provider retention, data use, region, and selected model support are verified before production rather than assumed from client code.            | Data lifecycle table; OpenAI data-controls research; Issue #3  |
| `PRIV-CREDENTIALS` | OpenAI/backend credentials never enter the Tauri bundle, React state, local client storage, diagnostics, or analytics.                           | FR-013; transcription boundary; Issue #3 secret management     |
| `PRIV-LOGGING`     | Raw audio, transcript text, authorization, signed locations, and credentials are excluded from default logs/analytics.                           | FR-014; transcription boundary; Issues #3–#7 review gate       |
| `SCOPE-DEFERRED`   | Background recording, realtime transcription, desktop recording, sync/auth, and optional VAD are not silently introduced.                        | FR-016; plan/research deferred scope                           |

### US3 deletion outcomes

- [x] Cancel before submission deletes the local temporary artifact after
      confirmation and creates no remote work.
- [x] Cancel after submission invalidates the logical operation, ignores late
      results, and schedules app-backend temporary cleanup.
- [x] Save with default choice commits one memo and deletes source audio.
- [x] Save with explicit retention attaches the original source to one memo until
      the user deletes the audio or memo.
- [x] Derived audio, if specified later, remains a distinct temporary artifact
      and never replaces the original implicitly.

## Success-Criteria Evidence Ownership

`SUCCESS-OWNERSHIP` distinguishes contract readiness from future runtime proof.

| Criterion                             | Issue #2 evidence                                                  | Runtime evidence owner                                                                    |
| ------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| SC-001 first-time mobile completion   | Prioritized P1 steps and physical-device procedure are complete.   | #4 native capture, #6 integration, #7 usability test; Android adapter issue also required |
| SC-002 one-second acknowledgment      | Observable state names and transition events are complete.         | #5 transition timing and #7 UI behavior tests                                             |
| SC-003 no duplicates in 100 trials    | Stable identities, idempotency, and duplicate guards are complete. | #3 backend, #5 domain, #6 integration, #7 repeated-action tests                           |
| SC-004 no silent loss/upload          | Recovery/termination outcomes and durable ownership are complete.  | #4 native and #6 relaunch/offline device tests; Android adapter issue required            |
| SC-005 physical iOS/Android gate      | Required matrix and evidence fields are complete.                  | #4/#6/#7 plus Android native implementation issue on physical devices                     |
| SC-006 zero secrets/sensitive logs    | Prohibition and inspection surfaces are complete.                  | Security inspection across #3–#7 before production                                        |
| SC-007 retention/deletion correctness | Every app-controlled artifact and terminal policy is complete.     | #3 cleanup, #6 lifecycle integration, #7 user-control tests                               |

## Validation Results

### Issue #2 acceptance

- [x] The specification has independent iOS and Android validation scenarios.
- [x] Every recording-to-memo state and recovery transition is defined.
- [x] Audio and transcript creation, transfer, retention, and deletion are
      explicit.
- [x] OpenAI credentials are prohibited from the Tauri client.
- [x] Background recording, realtime transcription, and desktop expansion are
      explicitly excluded.
- [x] Pre-research and post-design Constitution Checks pass with no exception.

### Executed checks

| Check                      | Result                                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------- |
| Incremental TDD            | US1, US2, and US3 each produced an expected RED before its readiness evidence and returned to GREEN |
| Focused contract test      | PASS — 4/4 tests                                                                                    |
| Full Vitest suite          | PASS — 6/6 tests                                                                                    |
| Production frontend build  | PASS — TypeScript and Vite production build                                                         |
| ESLint                     | PASS                                                                                                |
| Prettier                   | PASS after formatting normalization                                                                 |
| Rust tests                 | PASS — existing vendored `swift-rs` emitted two unrelated warnings                                  |
| Cross-artifact analysis    | PASS — 16/16 FR, 5/5 MLR, 4/4 PDL, 7/7 SC covered; no CRITICAL/HIGH findings                        |
| `git diff --check`         | PASS                                                                                                |
| Quickstart contract review | PASS — all six review steps and the future device matrix are present                                |

Physical-device execution is intentionally not marked complete: Issue #2 adds
no recorder runtime. The matrix is a mandatory gate for its owning follow-up
implementation issues.
