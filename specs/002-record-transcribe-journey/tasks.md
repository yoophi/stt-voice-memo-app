---
description: "Dependency-ordered implementation tasks for the record-and-transcribe journey contract"
---

# Tasks: Record and Transcribe Memo Journey Contract

**Input**: Design documents from `/specs/002-record-transcribe-journey/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, and `quickstart.md`

**Tests**: Issue #2 changes no production behavior. Its public output is the
contract package, so tests validate required artifacts, stable requirement/state
coverage, security exclusions, and downstream ownership. Runtime domain, port,
adapter, integration, UI, and physical-device tests are assigned to Issues #3–#7.

**Organization**: Tasks follow the three independently testable user journeys and
finish with a cross-artifact readiness gate.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it changes a different file and has no
  unmet dependency on another parallel task.
- **[Story]**: Maps the task to User Story 1, 2, or 3.
- Every task includes an exact repository path.

## Phase 1: Setup

**Purpose**: Pin Issue #2 as the active Spec Kit feature without changing runtime
permissions or source behavior.

- [x] T001 Verify `.specify/feature.json` selects
      `specs/002-record-transcribe-journey` and confirm `git diff --check` passes

---

## Phase 2: Foundational Contract Package

**Purpose**: Establish the shared decisions and semantic boundaries required by
all three stories.

- [x] T002 [P] Finalize primary-source decisions, rationale, alternatives, and
      deferred scope in `specs/002-record-transcribe-journey/research.md`
- [x] T003 [P] Finalize stable identities, entity invariants, lifecycle states,
      and state ownership in `specs/002-record-transcribe-journey/data-model.md`
- [x] T004 [P] Finalize foreground native semantics in
      `specs/002-record-transcribe-journey/contracts/recorder-port.md` and
      backend/idempotency/security semantics in
      `specs/002-record-transcribe-journey/contracts/transcription-boundary.md`
- [x] T005 Finalize canonical transitions, duplicate guards, relaunch rules, and
      error categories in
      `specs/002-record-transcribe-journey/contracts/journey-state-machine.md`
      using the decisions from T002–T004

**Checkpoint**: Every downstream issue has one canonical behavior and ownership
source before user-story readiness is asserted.

---

## Phase 3: User Story 1 - Turn a recording into an editable memo (Priority: P1) 🎯 MVP

**Goal**: Prove the contract completely describes the successful foreground
record-to-edit-to-save journey without exposing unfinished runtime controls.

**Independent Test**: The contract verification test traces one recording session
through one finalized source, one final transcript draft, one edited save, and
one memo, including the default audio-deletion result.

### Tests for User Story 1

- [x] T006 [US1] Write the contract-package test first in
      `scripts/record-transcribe-contract.test.mjs`; require a not-yet-created
      `checklists/implementation-readiness.md` and run the focused test to capture the
      expected RED result

### Implementation for User Story 1

- [x] T007 [US1] Create the primary-flow requirement/contract trace and readiness
      evidence in
      `specs/002-record-transcribe-journey/checklists/implementation-readiness.md`,
      then run the focused test to GREEN
- [x] T008 [US1] Verify the primary flow in
      `specs/002-record-transcribe-journey/spec.md`, `data-model.md`, and
      `contracts/journey-state-machine.md` uses the same identities, state names,
      final-result rule, edit semantics, save idempotency, and default deletion rule

**Checkpoint**: User Story 1 is independently specified and mechanically guarded.

---

## Phase 4: User Story 2 - Recover without losing a recording (Priority: P2)

**Goal**: Prove every permission, lifecycle, connectivity, duplicate, relaunch,
and partial-failure outcome has a valid recovery or terminal path.

**Independent Test**: The readiness evidence traces denial, interruption,
backgrounding, offline queue, uncertain timeout, retry, cancellation, late
result, and app termination to an explicit state/data outcome.

### Tests for User Story 2

- [x] T009 [US2] Extend `scripts/record-transcribe-contract.test.mjs` first to
      require all recovery scenario identifiers and run the focused test to confirm
      the new assertions fail before readiness evidence is updated

### Implementation for User Story 2

- [x] T010 [US2] Add the recovery trace and iOS/Android lifecycle ownership to
      `specs/002-record-transcribe-journey/checklists/implementation-readiness.md`
      and align any discrepancy in
      `specs/002-record-transcribe-journey/contracts/recorder-port.md` or
      `contracts/journey-state-machine.md`, then run the focused test to GREEN
- [x] T011 [US2] Finalize future physical-device recovery scenarios and required
      evidence fields in `specs/002-record-transcribe-journey/quickstart.md`

**Checkpoint**: User Stories 1 and 2 are independently specified and guarded.

---

## Phase 5: User Story 3 - Control retained voice data (Priority: P3)

**Goal**: Prove the default-delete, explicit-retain, cancellation, late-result,
backend cleanup, and memo deletion policies are complete and consistent.

**Independent Test**: The readiness evidence traces each artifact from creation
to every permitted terminal retention/deletion state, with no credential or
sensitive-content logging path.

### Tests for User Story 3

- [x] T012 [US3] Extend `scripts/record-transcribe-contract.test.mjs` first to
      require privacy lifecycle, credential prohibition, and deferred-scope evidence
      and run the focused test to confirm the new assertions fail before the evidence
      is updated

### Implementation for User Story 3

- [x] T013 [US3] Add privacy/data-lifecycle and scope-ownership evidence to
      `specs/002-record-transcribe-journey/checklists/implementation-readiness.md`,
      align `spec.md` and `contracts/transcription-boundary.md` if needed, and run the
      focused test to GREEN

**Checkpoint**: All three user stories are independently specified and guarded.

---

## Phase 6: Polish and Cross-Cutting Readiness

**Purpose**: Establish the final Issue #2 handoff and constitution evidence.

- [x] T014 Run the full `speckit-analyze` coverage/consistency/constitution pass
      across `specs/002-record-transcribe-journey/spec.md`, `plan.md`, `tasks.md`,
      `research.md`, `data-model.md`, `contracts/`, and `quickstart.md`; resolve any
      CRITICAL/HIGH issue before implementation completion
- [x] T015 Verify every Issue #2 acceptance criterion and all requirements
      checklist items in `specs/002-record-transcribe-journey/checklists/requirements.md`
      and `checklists/implementation-readiness.md`
- [x] T016 Run `pnpm exec vitest run scripts/record-transcribe-contract.test.mjs`,
      `pnpm build`, `git diff --check`, and the quickstart contract review; record the
      results in `specs/002-record-transcribe-journey/checklists/implementation-readiness.md`

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup** has no dependencies.
- **Foundational** depends on T001 and blocks all user stories.
- **US1** depends on T002–T005 and establishes the test/evidence seam.
- **US2** depends on the US1 test/evidence seam, not on US1 product behavior.
- **US3** depends on the same seam and follows US2 to avoid concurrent edits to
  the shared test and readiness files.
- **Polish** depends on all user stories.

### Parallel opportunities

- T002, T003, and T004 can run in parallel because they own separate files.
- Research for Issues #3 (backend) and #4 (native recorder) can proceed in
  parallel after T005, but implementation must preserve these contracts.
- The future Rust domain/use-case work in #5 can proceed in parallel with native
  recorder work in #4 after #3 fixes the backend wire contract.
- T006–T013 are intentionally sequential because they evolve the same public
  test/evidence seam through RED then GREEN for each story.

## Parallel Example

```text
Task A: T002 research decisions in research.md
Task B: T003 identities and invariants in data-model.md
Task C: T004 recorder and transcription semantic contracts
Then:   T005 reconcile them into the canonical state machine
```

## Implementation Strategy

### MVP first

1. Complete Setup and Foundational contracts.
2. Add the failing public contract-package test.
3. Complete US1 traceability until the test passes.
4. Stop here if only the primary flow needs review; no runtime behavior is
   implied.

### Incremental delivery

1. US1 locks the successful journey.
2. US2 adds recovery/lifecycle guarantees without changing success semantics.
3. US3 adds retention/deletion guarantees without changing operation identity.
4. Polish validates the whole handoff against Issue #2 and the constitution.

## Notes

- A checked task means its file evidence exists and validation ran; it does not
  claim that Issues #3–#7 are implemented.
- Physical-device scenarios are required acceptance procedures for follow-up
  implementations. Issue #2 changes no audio code, so it cannot honestly record
  device execution results.
- Commit/PR/issue closure is not part of these tasks unless separately requested.
