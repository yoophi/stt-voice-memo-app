# Tasks: Transcription Upload Use Case

**Input**: Design documents from `specs/004-transcription-upload-usecase/`
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: The specification explicitly requires automated domain, application,
HTTP contract, recovery, race, and content-safety tests. Test tasks precede the
corresponding implementation tasks.

**Organization**: Tasks are grouped by user story so each story remains an
independently testable increment.

## Phase 1: Setup

**Purpose**: Establish the compile-isolated core and root adapter structure.

- [x] T001 Add `transcription-core` to the workspace and create its manifest/module skeleton in `src-tauri/Cargo.toml`, `src-tauri/crates/transcription-core/Cargo.toml`, and `src-tauri/crates/transcription-core/src/lib.rs`
- [x] T002 Add the researched HTTP, async, streaming, serialization, hashing, and error dependencies to `src-tauri/Cargo.toml` and refresh `src-tauri/Cargo.lock`
- [x] T003 [P] Create root transcription adapter and inbound module skeletons in `src-tauri/src/infrastructure/mod.rs`, `src-tauri/src/infrastructure/transcription/mod.rs`, `src-tauri/src/inbound/mod.rs`, and `src-tauri/src/transcription_state.rs`
- [x] T004 [P] Create the content-safe physical-device evidence template in `tests/device/transcription-upload-usecase.md`

---

## Phase 2: Foundational Contracts

**Purpose**: Implement the shared domain, ports, and durable trusted-data seams
that block all user stories.

**⚠️ CRITICAL**: No user story work begins until this phase passes.

- [x] T005 Write failing identifier, descriptor, transcript, failure, phase-transition, progress-ordering, and first-terminal tests in `src-tauri/crates/transcription-core/tests/state_machine.rs`
- [x] T006 Implement validated value objects, operation aggregate, transition rules, and sanitized errors in `src-tauri/crates/transcription-core/src/domain.rs`
- [x] T007 Define object-safe async transcription, source-audio, repository, authorization, connectivity, clock, and event-sink contracts in `src-tauri/crates/transcription-core/src/ports.rs`
- [x] T008 Write failing atomic persistence, revision-conflict, content-exclusion, and relaunch-list tests in `src-tauri/src/infrastructure/transcription/local_operation_store.rs`
- [x] T009 Implement app-private temp-write/sync/rename operation persistence with revision compare-and-swap in `src-tauri/src/infrastructure/transcription/local_operation_store.rs`
- [x] T010 [P] Write failing source containment, metadata, size/duration, checksum, disappeared-file, and changed-file tests in `src-tauri/src/infrastructure/transcription/private_source_audio.rs`
- [x] T011 [P] Implement the trusted app-private source manifest/fixture adapter and per-attempt revalidation in `src-tauri/src/infrastructure/transcription/private_source_audio.rs`
- [x] T012 Implement redacted ephemeral access-token, deterministic clock/connectivity defaults, and shared infrastructure exports in `src-tauri/src/infrastructure/transcription/auth_session.rs` and `src-tauri/src/infrastructure/transcription/mod.rs`

**Checkpoint**: Core types and every external seam compile without Tauri, HTTP,
filesystem, or provider dependencies in `transcription-core`.

---

## Phase 3: User Story 1 — Submit finalized audio for transcription (Priority: P1) 🎯 MVP

**Goal**: Persist one intent, stream one validated source to the backend contract,
and resolve one authoritative final transcript.

**Independent Test**: A deterministic finalized fixture and backend double move
one stable operation through submit, queued/processing status, and completed
non-blank result; repeated submit/status returns the same operation/result.

### Tests for User Story 1

- [x] T013 [US1] Write failing submit/status, duplicate-submit, invalid-source, blank-result, malformed-state, and persistence-before-side-effect tests in `src-tauri/crates/transcription-core/tests/use_case.rs`
- [x] T014 [P] [US1] Write failing loopback HTTP tests for exact create/GET paths, Bearer and idempotency headers, multipart metadata/audio, 200/202 parsing, request IDs, and malformed responses in `src-tauri/tests/transcription_http_contract.rs`
- [x] T015 [P] [US1] Write failing camelCase command DTO and forbidden-field serialization tests in `src-tauri/src/inbound/transcription_commands.rs`

### Implementation for User Story 1

- [x] T016 [US1] Implement `TranscriptionService::submit` and `status`, source/options deduplication, durable intent, backend mapping, and completed-result validation in `src-tauri/crates/transcription-core/src/application.rs`
- [x] T017 [US1] Implement reusable HTTPS-only reqwest client, streamed multipart create, GET status, response/error mapping, and bounded timeouts in `src-tauri/src/infrastructure/transcription/http_backend.rs`
- [x] T018 [US1] Implement thin async submit/status commands and sanitized public DTO mapping in `src-tauri/src/inbound/transcription_commands.rs`
- [x] T019 [US1] Compose the long-lived transcription service and register submit/status commands in `src-tauri/src/transcription_state.rs` and `src-tauri/src/lib.rs`

**Checkpoint**: User Story 1 passes with fake/loopback backends and no OpenAI
credential, live microphone, or React UI.

---

## Phase 4: User Story 2 — Recover safely from mobile network failures (Priority: P2)

**Goal**: Preserve identity and recover offline, retryable, uncertain, and
relaunch states without duplicate provider work.

**Independent Test**: Offline, lost create response, known-ID timeout, 429/503,
stale progress, persistence failure, and relaunch at every non-terminal phase all
recover the same operation within one explicit action.

### Tests for User Story 2

- [x] T020 [US2] Write failing offline, exact create replay, GET-first resolution, Retry-After, bounded retry, and relaunch recovery tests in `src-tauri/crates/transcription-core/tests/recovery.rs`
- [x] T021 [P] [US2] Write failing streamed progress, monotonic sequence, stale-attempt suppression, timeout, and request-rebuild tests in `src-tauri/tests/transcription_http_contract.rs`
- [x] T022 [P] [US2] Add failure-injection tests for intent/CAS persistence failures around every network boundary in `src-tauri/crates/transcription-core/tests/recovery.rs`

### Implementation for User Story 2

- [x] T023 [US2] Implement `retry` and `recover`, online/offline projection, exact replay permission, GET-first resolution, retry eligibility, and CAS reconciliation in `src-tauri/crates/transcription-core/src/application.rs`
- [x] T024 [US2] Implement attempt-scoped bounded-memory progress streaming, throttling, timeout classification, and request rebuilding in `src-tauri/src/infrastructure/transcription/http_backend.rs`
- [x] T025 [US2] Add retry/recover commands and DTO mapping, then register them in `src-tauri/src/inbound/transcription_commands.rs` and `src-tauri/src/lib.rs`

**Checkpoint**: User Stories 1 and 2 pass independently and no uncertain outcome
causes a blind new logical operation.

---

## Phase 5: User Story 3 — Cancel and contain sensitive data (Priority: P3)

**Goal**: Make cancellation idempotent, preserve the first terminal winner, and
keep cleanup/errors/events free of sensitive content.

**Independent Test**: Cancellation before upload, during upload, during
processing, against completion, and with uncertain DELETE produces one stored
winner, no late transcript replacement, retryable cleanup, and zero content
canary leakage.

### Tests for User Story 3

- [x] T026 [US3] Write failing cancellation-before-dispatch, cancel/completion race, late-result, repeated-cancel, uncertain-DELETE, and cleanup-recovery tests in `src-tauri/crates/transcription-core/tests/use_case.rs`
- [x] T027 [P] [US3] Add loopback DELETE, local cancellation token, 202/204, timeout, and cleanup-state mapping tests in `src-tauri/tests/transcription_http_contract.rs`
- [x] T028 [P] [US3] Write token/path/audio/transcript/signed-URL/provider-payload canary tests for records, DTOs, errors, and events in `src-tauri/src/infrastructure/transcription/tauri_event_sink.rs`

### Implementation for User Story 3

- [x] T029 [US3] Implement `cancel`, persisted cancel intent, first-terminal CAS, late-result rejection, idempotent cleanup, and cleanup recovery in `src-tauri/crates/transcription-core/src/application.rs`
- [x] T030 [US3] Implement operation cancellation-token registry and idempotent backend DELETE mapping in `src-tauri/src/infrastructure/transcription/http_backend.rs`
- [x] T031 [US3] Implement throttled content-safe `transcription://event` emission plus cancel command registration in `src-tauri/src/infrastructure/transcription/tauri_event_sink.rs`, `src-tauri/src/inbound/transcription_commands.rs`, and `src-tauri/src/lib.rs`

**Checkpoint**: All three stories pass and every terminal ordering preserves one
winner without observable sensitive content.

---

## Phase 6: Polish & Cross-Cutting Validation

**Purpose**: Prove contract conformance, architecture, mobile builds, and
completion evidence.

- [x] T032 [P] Add a repository-level artifact/architecture contract test for Issue #5 boundaries and forbidden fields in `scripts/transcription-upload-contract.test.mjs`
- [x] T033 Run and fix focused OpenAPI/journey/new artifact tests and document results in `specs/004-transcription-upload-usecase/quickstart.md`
- [ ] T034 Run and fix Prettier, ESLint, TypeScript, Rust fmt, workspace tests, and clippy commands from `specs/004-transcription-upload-usecase/quickstart.md`
- [ ] T035 Build the Rust/Tauri feature for iOS simulator/device and Android arm64 targets and record non-physical evidence in `tests/device/transcription-upload-usecase.md`
- [ ] T036 Validate success plus offline/uncertain recovery on a physical iPhone and record content-safe evidence in `tests/device/transcription-upload-usecase.md`
- [ ] T037 Validate success plus offline/uncertain recovery on a physical Android API 24+ device and record content-safe evidence in `tests/device/transcription-upload-usecase.md`
- [x] T038 Reconcile every requirement, contract, Constitution gate, and completed task against implementation evidence in `specs/004-transcription-upload-usecase/quickstart.md` and `specs/004-transcription-upload-usecase/tasks.md`

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)** has no prerequisites.
- **Foundational (Phase 2)** depends on Setup and blocks every user story.
- **US1 (Phase 3)** depends on Foundational and is the MVP.
- **US2 (Phase 4)** depends on US1's stable operation and backend mapping but is
  independently testable through fake ports and stored fixtures.
- **US3 (Phase 5)** depends on the shared aggregate/transport from US1 and CAS
  recovery from US2; its terminal-race tests remain independently runnable.
- **Polish (Phase 6)** depends on all implemented user stories. Physical tasks
  require connected devices and must never be marked complete from simulator or
  compile-only evidence.

### User story dependency graph

```text
Setup -> Foundational -> US1 (MVP) -> US2 -> US3 -> Polish
```

### Within each user story

- Write each named failing test before its corresponding implementation.
- Domain/application policy precedes infrastructure and inbound composition.
- Persist intent/state before network side effects and emit events only after
  durable commits.
- Files shared by multiple tasks are changed sequentially in task-ID order.

## Parallel Opportunities

- T003 and T004 can run in parallel after T001/T002 establish paths/dependencies.
- T010/T011 can proceed alongside T008/T009 because source and operation stores
  are separate files after ports exist.
- US1 contract tests T014/T015 can run in parallel after T013 defines expected
  application behavior.
- US2 HTTP progress tests T021 can run alongside repository failure tests T022.
- US3 HTTP cancellation tests T027 can run alongside content-canary tests T028.
- T032 can run alongside documentation/evidence preparation after source layout
  stabilizes.

## Parallel Example: User Story 1

```text
Task T014: Write HTTP create/status wire tests in src-tauri/tests/transcription_http_contract.rs
Task T015: Write command DTO safety tests in src-tauri/src/inbound/transcription_commands.rs
```

## Parallel Example: User Story 2

```text
Task T021: Write progress/timeout/rebuild HTTP tests in src-tauri/tests/transcription_http_contract.rs
Task T022: Write persistence failure-injection tests in src-tauri/crates/transcription-core/tests/recovery.rs
```

## Parallel Example: User Story 3

```text
Task T027: Write DELETE/cancellation wire tests in src-tauri/tests/transcription_http_contract.rs
Task T028: Write content-canary tests in src-tauri/src/infrastructure/transcription/tauri_event_sink.rs
```

## Implementation Strategy

### MVP first

1. Complete Setup and Foundational phases.
2. Complete User Story 1 tests and implementation.
3. Run core, HTTP contract, architecture, and standard quality checks.
4. Demonstrate one deterministic fixture reaching one final transcript through a
   backend double before adding recovery/cancellation breadth.

### Incremental delivery

1. US1 establishes one safe submit/status/completed result.
2. US2 adds offline, exact replay, retry, progress, and relaunch recovery.
3. US3 adds cancellation races, cleanup, and event/content safety.
4. Polish proves the complete contract and records physical-device evidence.

## Notes

- Do not add a WebView-visible arbitrary URL/path/token upload command.
- Do not persist transcript text in the operation store; Issue #7 owns it.
- Do not wire recorder artifacts or add production React state in this issue;
  Issue #6 owns both.
- Do not mark T036 or T037 complete without actual physical-device evidence.
