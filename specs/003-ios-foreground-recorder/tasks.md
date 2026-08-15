# Tasks: iOS Foreground Recorder Adapter

**Input**: Design documents from `/specs/003-ios-foreground-recorder/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/recorder-plugin.md`, `quickstart.md`

**Tests**: Test-first work is required for the pure Rust contract, native Swift
coordinator/lifecycle behavior, TypeScript client surface, privacy-sensitive
mapping, and physical-iPhone acceptance flows.

**Organization**: Tasks are grouped by user story so the normal capture MVP,
disruption handling, and cancellation/privacy slice remain independently
testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it changes different files and has no
  dependency on an incomplete task in the same phase.
- **[Story]**: Maps work to US1, US2, or US3 from `spec.md`.
- Every task names its concrete repository path.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish the local pure-core and Tauri mobile-plugin structure.

- [x] T001 Configure the Rust workspace and scaffold `src-tauri/crates/recorder-core/Cargo.toml`, `src-tauri/crates/recorder-core/src/lib.rs`, and `src-tauri/plugins/recorder/` from the Tauri 2 iOS plugin template
- [x] T002 [P] Create the content-safe physical-device evidence matrix in `tests/device/ios-foreground-recorder.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define the platform-neutral contract and least-privilege plugin
boundary required by every user story.

**⚠️ CRITICAL**: No user story implementation begins before this phase passes.

- [x] T003 Write failing domain transition, session identity, descriptor validation, and fake-port orchestration tests in `src-tauri/crates/recorder-core/src/lib.rs`
- [x] T004 Implement recorder domain IDs, permission/state/error/event/descriptor/cleanup models and validation in `src-tauri/crates/recorder-core/src/domain.rs`
- [x] T005 Implement the platform-neutral `RecorderPort` and application recorder service with transition and terminal-result rules in `src-tauri/crates/recorder-core/src/ports.rs` and `src-tauri/crates/recorder-core/src/application.rs`
- [x] T006 [P] Write failing IPC serialization and sanitized error mapping tests in `src-tauri/plugins/recorder/src/models.rs` and `src-tauri/plugins/recorder/src/error.rs`
- [x] T007 Implement the plugin build command list, Rust facade, mobile/desktop adapters, and thin commands in `src-tauri/plugins/recorder/build.rs` and `src-tauri/plugins/recorder/src/`
- [x] T008 Configure individual recorder command permissions and microphone-only host declarations in `src-tauri/plugins/recorder/permissions/default.toml`, `src-tauri/capabilities/default.json`, and `src-tauri/gen/apple/stt-voice-memo-app_iOS/Info.plist`

**Checkpoint**: The pure Rust core tests without Tauri/iOS, the desktop
unsupported adapter, and generated command permissions all compile.

---

## Phase 3: User Story 1 - Record and finalize a voice memo on iPhone (Priority: P1) 🎯 MVP

**Goal**: Start, pause, resume, and stop one foreground iPhone recording and
return one verified public descriptor.

**Independent Test**: Grant microphone access on a physical iPhone, record a
known phrase, pause/resume once, and stop; obtain one playable M4A with positive
duration/bytes and an inactive audio session.

### Tests for User Story 1

- [x] T009 [P] [US1] Write failing Swift permission/start/pause/resume/stop and metadata tests with injected fakes in `src-tauri/plugins/recorder/ios/Tests/PluginTests/RecorderCoordinatorTests.swift`
- [x] T010 [P] [US1] Write failing TypeScript command-name and public-descriptor redaction tests in `src/shared/api/recorder/recorder-client.test.ts`

### Implementation for User Story 1

- [x] T011 [P] [US1] Define Codable native request/result/error/event types with no public raw path field in `src-tauri/plugins/recorder/ios/Sources/RecorderTypes.swift`
- [x] T012 [US1] Implement the injected recorder engine and main-actor coordinator for app-private AAC/M4A start, pause, resume, stop, audio-session activation/deactivation, and metadata in `src-tauri/plugins/recorder/ios/Sources/RecorderEngine.swift`
- [x] T013 [US1] Bind permission, status, start, pause, resume, and stop Swift plugin invokes to the coordinator in `src-tauri/plugins/recorder/ios/Sources/RecorderPlugin.swift`
- [x] T014 [US1] Complete Rust mobile delegation and private-locator-to-public-descriptor validation for normal capture in `src-tauri/plugins/recorder/src/mobile.rs`, `src-tauri/plugins/recorder/src/commands.rs`, and `src-tauri/plugins/recorder/src/models.rs`
- [x] T015 [US1] Implement and export the typed shared recorder client in `src/shared/api/recorder/recorder-client.ts` and `src/shared/api/recorder/index.ts`
- [x] T016 [US1] Register the recorder plugin with the Tauri application and workspace path dependencies in `src-tauri/src/lib.rs` and `src-tauri/Cargo.toml`
- [ ] T017 [US1] Build the iOS target and record the normal 20-run, pause/resume, and five-cold-launch physical-iPhone results in `tests/device/ios-foreground-recorder.md`

**Checkpoint**: US1 can be demonstrated independently without transcription or
memo UI.

---

## Phase 4: User Story 2 - Handle denial and audio disruptions predictably (Priority: P2)

**Goal**: Normalize denied permission and end interrupted, route-changed,
media-reset, or backgrounded capture exactly once without auto-resume.

**Independent Test**: Exercise each lifecycle event separately on a physical
iPhone; every case returns one sanitized terminal reason and leaves no active
capture or audio session.

### Tests for User Story 2

- [x] T018 [US2] Add failing Swift race tests for denial, interruption, route loss, media reset, background entry, and concurrent user stop in `src-tauri/plugins/recorder/ios/Tests/PluginTests/RecorderCoordinatorTests.swift`
- [x] T019 [P] [US2] Add failing TypeScript native-event decoding, deduplication-field, and redaction tests in `src/shared/api/recorder/recorder-client.test.ts`

### Implementation for User Story 2

- [x] T020 [US2] Implement iOS 15–16 and iOS 17+ permission mapping plus `.record` audio-session cleanup across every failure path in `src-tauri/plugins/recorder/ios/Sources/RecorderEngine.swift`
- [x] T021 [US2] Observe interruption, risky input-route change, media-services reset, and application background notifications through one terminal gate in `src-tauri/plugins/recorder/ios/Sources/RecorderEngine.swift`
- [x] T022 [US2] Emit plugin-scoped sanitized `recorderEvent` payloads for native lifecycle outcomes in `src-tauri/plugins/recorder/ios/Sources/RecorderPlugin.swift`
- [x] T023 [US2] Expose typed recorder-event subscription/unsubscription without adding a global state store in `src/shared/api/recorder/recorder-client.ts`
- [ ] T024 [US2] Record permission denial, interruption, route change, media reset, foreground exit, and no-auto-resume physical-iPhone results in `tests/device/ios-foreground-recorder.md`

**Checkpoint**: US1 and US2 each have independent normal and disruption
evidence, and neither needs Issue #6 UI reconciliation.

---

## Phase 5: User Story 3 - Cancel safely and leave no abandoned audio (Priority: P3)

**Goal**: Make cancel and repeated terminal actions idempotent, delete temporary
audio, and expose cleanup-pending/failure without path leakage.

**Independent Test**: Cancel active and paused sessions, repeat cancel/stop, and
inject deletion failure; verify one outcome, no artifact after successful
cleanup, and a sanitized retryable cleanup result otherwise.

### Tests for User Story 3

- [x] T025 [US3] Add failing Rust fake-port tests for cancel idempotency, stop/cancel conflict, cleanup pending/failure, and terminal-result reuse in `src-tauri/crates/recorder-core/src/lib.rs`
- [x] T026 [US3] Add failing Swift active/paused cancel, repeated terminal trigger, and deletion-failure tests in `src-tauri/plugins/recorder/ios/Tests/PluginTests/RecorderCoordinatorTests.swift`

### Implementation for User Story 3

- [x] T027 [US3] Implement coordinator cancel, temporary artifact deletion, and stored terminal cleanup outcomes in `src-tauri/plugins/recorder/ios/Sources/RecorderEngine.swift`
- [x] T028 [US3] Complete Swift/Rust/TypeScript cancel mapping and idempotent cleanup result handling in `src-tauri/plugins/recorder/ios/Sources/RecorderPlugin.swift`, `src-tauri/plugins/recorder/src/`, and `src/shared/api/recorder/recorder-client.ts`
- [ ] T029 [US3] Record active/paused cancellation, repeated stop/cancel, artifact deletion, and audio-session cleanup physical-iPhone results in `tests/device/ios-foreground-recorder.md`

**Checkpoint**: All three user stories are independently functional and expose
only contract-safe results.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Prove architecture, security, build quality, and documented
completion evidence across all stories.

- [x] T030 Run and fix repository formatting, ESLint, TypeScript, Rust workspace test, clippy, and iOS debug build commands documented in `specs/003-ios-foreground-recorder/quickstart.md`
- [x] T031 Verify dependency direction, individual command capability scope, absence of background audio entitlement, and absence of raw path/audio/native error logging across `src-tauri/`, `src/shared/api/recorder/`, and `src-tauri/gen/apple/stt-voice-memo-app_iOS/Info.plist`
- [x] T032 Reconcile requirement and acceptance evidence, document any unavailable physical-device checks, and finalize validation notes in `tests/device/ios-foreground-recorder.md` and `specs/003-ios-foreground-recorder/quickstart.md`

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)** has no dependencies.
- **Foundational (Phase 2)** depends on Setup and blocks every user story.
- **US1 (Phase 3)** depends on Foundational and is the MVP.
- **US2 (Phase 4)** depends on the US1 Swift coordinator and client event
  surface, while its denial/lifecycle tests remain independently runnable.
- **US3 (Phase 5)** depends on the shared terminal gate from US2, while its
  cleanup contract remains independently runnable.
- **Polish (Phase 6)** depends on all implemented story phases.

### Within each user story

- Write the listed tests and confirm they fail before implementation.
- Implement domain/types before adapters, and adapters before integration.
- Files shared by multiple tasks are changed sequentially in task-ID order.
- Do not mark physical-device tasks complete without actual device evidence.

### Parallel opportunities

- T002 may run alongside T001.
- T006 may run alongside T003–T005 before T007 integrates the plugin.
- T009 and T010 may run in parallel; T011 may proceed alongside T009.
- T019 may run alongside T018.
- Documentation/evidence preparation can proceed independently, but device
  result tasks remain sequential with their completed implementation.

## Parallel Example: User Story 1

```text
Task T009: Write Swift coordinator tests in RecorderCoordinatorTests.swift
Task T010: Write TypeScript public API tests in recorder-client.test.ts
Task T011: Define native Codable types in RecorderTypes.swift
```

## Implementation Strategy

### MVP first

1. Complete Setup and Foundational phases.
2. Complete US1 tests and implementation.
3. Run the iOS build and physical normal-recording matrix.
4. Stop here for an independently demonstrable native recording MVP if needed.

### Incremental delivery

1. US1 adds valid start/pause/resume/stop and a finalized descriptor.
2. US2 adds deterministic permission/lifecycle disruption outcomes.
3. US3 adds cancellation privacy and terminal idempotency.
4. Polish verifies the complete Issue #4 contract without introducing Issue #6
   UI or transcription behavior.

## Notes

- Physical iPhone evidence is a constitution and issue acceptance gate; simulator
  or host tests are supplementary.
- The plugin may return an internal file URI to trusted Rust adapter code only;
  React, logs, and analytics receive no absolute path.
- Do not add Android, background audio, upload, transcription, VAD, or memo UI.
- Mark every completed task `[X]` only after its named verification passes.
