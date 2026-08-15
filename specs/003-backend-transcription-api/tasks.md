---
description: "Dependency-ordered tasks for the backend transcription API contract"
---

# Tasks: Backend Transcription API Contract

**Input**: Design documents from `/specs/003-backend-transcription-api/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/http-api.md`, `contracts/error-catalog.md`, and `quickstart.md`

**Tests**: TDD is required. Each story extends the public contract test first,
observes RED, then adds only the OpenAPI contract needed for GREEN. Tests call no
backend or OpenAI and mock no internal collaborator.

**Organization**: Tasks are grouped by the three independently reviewable
contract stories.

## Format: `[ID] [P?] [Story?] Description with exact file path`

## Phase 1: Setup

**Purpose**: Confirm the active feature and create only contract/test directories.

- [x] T001 Verify `.specify/feature.json` selects
      `specs/003-backend-transcription-api`, preserve `.wtp.yml`, and create
      `contracts/transcription-api/v1/` without changing production source
- [x] T002 Verify existing Vitest discovery includes
      `scripts/backend-transcription-api-contract.test.mjs` via `scripts/*.test.mjs`
      in `vitest.config.ts` and add no new dependency

---

## Phase 2: Foundational Contract Semantics

**Purpose**: Lock the shared vocabulary and policies before machine-readable
implementation.

- [x] T003 [P] Validate resource states, fingerprints, problem fields, cleanup,
      and retention invariants in
      `specs/003-backend-transcription-api/data-model.md`
- [x] T004 [P] Validate path/header/status/idempotency/limit semantics in
      `specs/003-backend-transcription-api/contracts/http-api.md`
- [x] T005 [P] Validate every HTTP/code/category/retry mapping in
      `specs/003-backend-transcription-api/contracts/error-catalog.md`

**Checkpoint**: One semantic source exists for every OpenAPI assertion.

---

## Phase 3: User Story 1 - Submit audio and retrieve a final transcript (Priority: P1) 🎯 MVP

**Goal**: Publish a parseable versioned contract for authenticated asynchronous
multipart creation and provider-neutral status/final-result retrieval.

**Independent Test**: Parse the contract, resolve all local references, submit
the documented success example conceptually, and verify POST 202 and GET 200
represent one operation whose transcript appears only when completed.

### Tests for User Story 1

- [x] T006 [US1] Create
      `scripts/backend-transcription-api-contract.test.mjs` first with artifact,
      OpenAPI 3.1.1, local-reference, Bearer security, multipart/header, POST 202,
      GET 200, state, and completed-result assertions; run the focused test and
      capture expected RED because
      `contracts/transcription-api/v1/openapi.json` does not exist

### Implementation for User Story 1

- [x] T007 [US1] Create the minimal OpenAPI document, server-neutral metadata,
      Bearer scheme, common headers, POST `/v1/transcriptions`, and GET
      `/v1/transcriptions/{operationId}` in
      `contracts/transcription-api/v1/openapi.json`
- [x] T008 [US1] Add multipart request, operation/result/cleanup/link schemas,
      queued/processing/completed examples, 202 Location/Retry-After/no-store, and
      completed GET behavior in `contracts/transcription-api/v1/openapi.json`, then
      run the focused test to GREEN

**Checkpoint**: The P1 contract is independently machine-readable and testable.

---

## Phase 4: User Story 2 - Retry or cancel safely (Priority: P2)

**Goal**: Add deterministic replay, conflict, failure, timeout, cancellation,
late-result, deletion, and expiry behavior.

**Independent Test**: Verify the contract represents same-key replay without a
new logical operation, changed-fingerprint rejection, uncertain status recovery,
typed retry guidance, idempotent DELETE, and expired tombstones.

### Tests for User Story 2

- [x] T009 [US2] Extend
      `scripts/backend-transcription-api-contract.test.mjs` first with idempotency
      replay/conflict, problem shape/category, Retry-After, DELETE 202/204, 410
      expiry, cancellation, and late-result guards; run focused test to RED

### Implementation for User Story 2

- [x] T010 [US2] Add replay 200/202 headers, DELETE semantics, failed/cancelled/
      deleting/deleted schemas and examples, RFC 9457 Problem schema, and 409/410/
      422/429/500/503/504 responses to
      `contracts/transcription-api/v1/openapi.json`
- [x] T011 [US2] Add every recovery error example from
      `specs/003-backend-transcription-api/contracts/error-catalog.md`, align each
      code/category/retry/status tuple in
      `contracts/transcription-api/v1/openapi.json`, and run focused test to GREEN

**Checkpoint**: US1 and US2 are independently represented and guarded.

---

## Phase 5: User Story 3 - Enforce privacy and usage boundaries (Priority: P3)

**Goal**: Complete owner isolation, pre-dispatch validation/limits, sensitive
data exclusions, retention, and cleanup policy in the canonical contract.

**Independent Test**: Verify every operation is authenticated, no user/provider/
model/key field crosses the boundary, all rejection classes have examples,
limits and retention are machine-readable, and fixtures contain no secret or
transcript content.

### Tests for User Story 3

- [x] T012 [US3] Extend
      `scripts/backend-transcription-api-contract.test.mjs` first with ownership-safe
      404, 400/401/403/413/415/all 422/429 examples, policy limits, 24-hour content
      deletion, seven-day tombstone, forbidden-field/string, and zero-external-ref
      assertions; run focused test to RED

### Implementation for User Story 3

- [x] T013 [US3] Add all validation/auth/ownership/limit examples and response
      references plus root `x-contract-policy` limits, retention, logging, provider,
      and test-boundary metadata in
      `contracts/transcription-api/v1/openapi.json`
- [x] T014 [US3] Complete the security/privacy scan, local `$ref` resolution, and
      named example coverage in `contracts/transcription-api/v1/openapi.json`, then
      run focused test to GREEN
- [x] T015 [US3] Create Issue #3 acceptance, requirement, security, lifecycle,
      and downstream evidence mapping in
      `specs/003-backend-transcription-api/checklists/implementation-readiness.md`

**Checkpoint**: All three contract stories are independently represented and
guarded without a provider call.

---

## Phase 6: Polish and Cross-Cutting Validation

**Purpose**: Prove complete, consistent, handoff-ready contract delivery.

- [x] T016 Run the read-only Spec Kit coverage/consistency/constitution analysis
      across `specs/003-backend-transcription-api/spec.md`, `plan.md`, and `tasks.md`;
      resolve any CRITICAL/HIGH finding before completion
- [x] T017 Run `pnpm exec vitest run
scripts/backend-transcription-api-contract.test.mjs`, full Vitest, ESLint,
      Prettier, frontend build, Rust tests, and `git diff --check`; record results in
      `specs/003-backend-transcription-api/checklists/implementation-readiness.md`
- [x] T018 Execute all seven manual review steps in
      `specs/003-backend-transcription-api/quickstart.md`, confirm no production or
      dependency changes and `.wtp.yml` remains untouched, then mark all completed
      tasks in `specs/003-backend-transcription-api/tasks.md`

---

## Dependencies & Execution Order

### Phase dependencies

- Setup has no dependency.
- Foundational depends on Setup and blocks story implementation.
- US1 depends on Foundational and creates the public test/OpenAPI seam.
- US2 depends on US1 because it extends the same test and contract files.
- US3 depends on US2 for the same reason.
- Polish depends on all stories.

### Parallel opportunities

- T003, T004, and T005 can run in parallel because they own separate design files.
- No story implementation task is marked `[P]`: each TDD slice updates the same
  test or OpenAPI artifact and must remain sequential.
- After this contract is complete, Issue #4 native iOS recording can continue in
  parallel with a separately specified production backend implementation.

## Parallel Example

```text
Task A: T003 validate data-model invariants
Task B: T004 validate HTTP operation semantics
Task C: T005 validate typed error catalog
Then:   T006 begin the first RED contract slice
```

## Implementation Strategy

### MVP first

1. Complete Setup and Foundational semantics.
2. Run T006 RED.
3. Complete T007–T008 and demonstrate US1 GREEN.
4. Stop here if only submit/status contract review is desired; no backend exists.

### Incremental delivery

1. US1 establishes accepted operation and final result.
2. US2 adds safe recovery and deletion without changing US1 shapes.
3. US3 adds security/privacy/usage gates without changing operation identity.
4. Polish proves the whole contract package and downstream handoff.

## Notes

- A checked task means contract evidence exists; it does not claim a production
  backend or OpenAI integration is deployed.
- Never add a credential, real transcript, signed location, provider endpoint, or
  real audio fixture to examples/tests.
- Commit, push, PR, and Issue closure require separate user requests.
