# Tasks: Backend Monorepo Workspace

**Input**: Design documents from `specs/005-backend-monorepo-workspace/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Workspace, dependency, contract drift, path selection, secret boundary,
and mobile-path preservation are security or architecture contracts and MUST be
implemented test-first. Physical iOS and Android migration evidence is excluded
from PR #22 and owned by follow-up GitHub Issue #23 as a separate release gate.

**Organization**: Tasks are grouped by user story and ordered for sequential
execution. `[P]` marks tasks that use different files and have no dependency on
another incomplete task in the same phase.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish tracked workspace roots without moving the mobile app.

- [x] T001 Create the backend reservation and canonical contract package directories in `apps/backend/` and `contracts/package.json`
- [x] T002 Add the pnpm workspace declaration for root, backend, and contract packages in `pnpm-workspace.yaml`
- [x] T003 Promote the Rust packages into one virtual workspace in `Cargo.toml`, `Cargo.lock`, and `src-tauri/Cargo.toml`
- [x] T004 [P] Create workspace validation script and fixture directories in `scripts/workspace/fixtures/`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define machine-readable ownership and shared test helpers before
implementing story-specific commands.

**⚠️ CRITICAL**: No user story work begins until this phase passes.

- [x] T005 Write failing ownership-map and root-workspace membership tests in `scripts/workspace/workspace-contract.test.mjs`
- [x] T006 Define the mobile, backend, contract, shared, and unknown ownership rules in `scripts/workspace/workspace-map.mjs`
- [x] T007 Implement content-safe filesystem/test helpers in `scripts/workspace/test-support.mjs`
- [x] T008 Add automated Tauri Apple/Android project path assertions to `scripts/workspace/workspace-contract.test.mjs`
- [x] T009 Create the physical migration evidence template with all scenarios marked Not run in `tests/device/backend-monorepo-workspace.md`

**Checkpoint**: Ownership and path contracts are executable; story work may begin.

---

## Phase 3: User Story 1 - Work in one clearly owned repository (Priority: P1) 🎯 MVP

**Goal**: Contributors can discover each area and execute honest scoped/full root commands.

**Independent Test**: From the root, list both workspace systems, run each
non-development scoped validation command, and confirm `dev:backend` explicitly
reports unavailable rather than starting or passing a fake runtime.

### Tests for User Story 1

- [x] T010 [US1] Extend failing tests for exact root command names, exit semantics, and workspace ownership in `scripts/workspace/workspace-contract.test.mjs`
- [x] T011 [P] [US1] Add boundary violation fixtures for mobile-to-backend, backend-to-mobile, and duplicate contract sources in `scripts/workspace/fixtures/boundaries/`
- [x] T012 [US1] Add failing boundary enforcement cases using the fixtures in `scripts/workspace/workspace-contract.test.mjs`

### Implementation for User Story 1

- [x] T013 [US1] Define the backend scoped command facade without a runtime dependency in `apps/backend/package.json`
- [x] T014 [P] [US1] Define the canonical contract workspace package exports and commands in `contracts/package.json`
- [x] T015 [US1] Implement dependency and duplicate-contract enforcement in `scripts/workspace/check-boundaries.mjs`
- [x] T016 [US1] Add mobile/backend/contract/full command facades and keep the existing Tauri facade in `package.json`
- [x] T017 [US1] Document repository ownership, dependency direction, local setup, root commands, and module/adapter contribution rules in `docs/monorepo-workspace.md`
- [x] T018 [US1] Document the reserved backend hexagonal target tree and explicit runtime unavailability in `apps/backend/README.md`

**Checkpoint**: User Story 1 is independently runnable and documented.

---

## Phase 4: User Story 2 - Share one safe transcription contract (Priority: P2)

**Goal**: Both consumers use one reproducible contract while backend-only names and canaries cannot enter the client.

**Independent Test**: Contract drift and synthetic client-secret fixtures fail;
the canonical contract regenerates deterministically and a clean mobile build scans cleanly.

### Tests for User Story 2

- [x] T019 [P] [US2] Add failing deterministic generation, missing output, and manual drift cases to `scripts/workspace/workspace-contract.test.mjs`
- [x] T020 [P] [US2] Add safe and leaking mobile output fixtures in `scripts/workspace/fixtures/client-secrets/`
- [x] T021 [US2] Add failing backend-name and caller-canary leak cases to `scripts/workspace/workspace-contract.test.mjs`

### Implementation for User Story 2

- [x] T022 [US2] Implement deterministic generate/check modes in `scripts/workspace/contract-artifacts.mjs`
- [x] T023 [US2] Generate the tracked source hash manifest in `contracts/transcription-api/v1/generated/contract-manifest.json`
- [x] T024 [P] [US2] Add names-only backend configuration guidance in `apps/backend/.env.example`
- [x] T025 [US2] Implement template-derived backend-name and synthetic-canary scanning in `scripts/workspace/check-client-secrets.mjs`
- [x] T026 [US2] Wire contract generation, drift, boundary, and client-secret commands into `package.json`, `apps/backend/package.json`, and `contracts/package.json`

**Checkpoint**: User Story 2 detects drift and secret boundary violations with no real credential.

---

## Phase 5: User Story 3 - Validate affected areas and preserve mobile behavior (Priority: P3)

**Goal**: Changed paths select minimal correct CI scopes while full and physical mobile validation remain available.

**Independent Test**: The four representative path trials match the contract,
the workflow consumes the same selector outputs, full validation runs, and mobile
project paths remain discoverable.

### Tests for User Story 3

- [x] T027 [P] [US3] Add failing table-driven mobile/backend/contract/root/unknown/empty path classification tests to `scripts/workspace/workspace-contract.test.mjs`
- [x] T028 [P] [US3] Add failing workflow structure, conditional job, cache-scope, and aggregate-result tests to `scripts/workspace/workspace-contract.test.mjs`

### Implementation for User Story 3

- [x] T029 [US3] Implement deterministic JSON and GitHub-output path classification in `scripts/workspace/select-scopes.mjs`
- [x] T030 [US3] Add changed-path selection plus conditional mobile, backend, contract, aggregate, and manual full jobs in `.github/workflows/validate.yml`
- [x] T031 [US3] Use scope-specific pnpm/Cargo cache keys and no production secrets in `.github/workflows/validate.yml`
- [x] T032 [US3] Wire `select:scopes`, scoped validation, and full validation commands in `package.json`
- [x] T033 [US3] Add automated mobile path and permission/config regression checks to `scripts/workspace/check-mobile-paths.mjs`

**Checkpoint**: User Story 3 is independently testable with local fixtures and workflow inspection.

---

## Phase 6: Polish & Cross-Cutting Validation

**Purpose**: Reconcile documentation, ignore rules, generated files, and all completion evidence.

- [x] T034 [P] Update `.gitignore`, `.prettierignore`, and `eslint.config.js` for root Cargo output, backend local environments, deterministic fixtures, and generated native output
- [x] T035 [P] Update root onboarding and scoped/full validation links in `README.md` and current feature context in `AGENTS.md`
- [x] T036 Run contract, backend, mobile, frontend, Rust, Clippy, Swift, formatting, lint, build, drift, and secret checks from `specs/005-backend-monorepo-workspace/quickstart.md`
- [x] T037 Transfer the physical iPhone build/install/launch regression and evidence ownership to GitHub Issue #23
- [x] T038 Transfer the physical Android build/install/launch regression and evidence ownership to GitHub Issue #23
- [x] T039 Review every changed file for real secrets, audio/transcript content, generated drift, and unintended mobile path changes, then record the automated results in `tests/device/backend-monorepo-workspace.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Starts immediately; T003 is sequential because lockfile
  ownership changes atomically.
- **Foundational (Phase 2)**: Depends on Phase 1 and blocks all user stories.
- **US1 (Phase 3)**: Depends on ownership helpers from Phase 2.
- **US2 (Phase 4)**: Depends on US1 package ownership and command facades.
- **US3 (Phase 5)**: Depends on US1/US2 command names so CI invokes stable scopes.
- **Polish (Phase 6)**: Depends on all automated implementation stories. T037 and
  T038 complete the scope handoff only; Issue #23 requires connected physical
  devices/signing and cannot be substituted by CI.

### User Story Dependencies

- **US1 (P1)**: Foundational only; delivers the MVP repository and command map.
- **US2 (P2)**: Uses the packages established by US1 but remains independently
  testable with contract/secret fixtures.
- **US3 (P3)**: Uses scoped commands from US1/US2 and independently proves path selection.

### Within Each User Story

1. Write the named failing tests and fixtures.
2. Run the focused test and confirm failure is caused by missing behavior.
3. Implement the smallest command/module satisfying the contract.
4. Re-run the focused story tests before proceeding.

## Parallel Opportunities

- T004 can run alongside T001-T003 after paths are agreed.
- T009 is independent of T005-T008.
- T011 and T014 touch different fixture/package files.
- T019 and T020 can be written in parallel; T024 is independent after the backend package exists.
- T027 and T028 are separate test tables in one file and should be edited sequentially by one contributor, but conceptually validate independent rules.
- T034 and T035 touch separate configuration/documentation files.
- Issue #23's physical iPhone and Android trials can run in parallel on separate
  equipped hosts after PR #22 is merged.

## Parallel Example: User Story 2

```text
Task T019: Add deterministic contract generation/drift tests.
Task T020: Add safe and leaking mobile output fixtures.
Task T024: Add the names-only backend environment template after US1 creates the package.
```

## Implementation Strategy

### MVP First (User Story 1)

1. Complete Setup and Foundational phases.
2. Implement US1 root workspaces, honest commands, boundaries, and contributor map.
3. Run `pnpm test:workspace` and scoped scaffold validation.
4. Stop point: Issue #12 can add a backend application module without moving mobile code.

### Incremental Delivery

1. US1 establishes ownership and commands.
2. US2 protects contract and secrets.
3. US3 adds selective CI and mobile migration checks.
4. Automated full validation completes in PR #22; Issue #23 owns later
   physical-device evidence against the merged revision.

## Notes

- Never place a real credential in a fixture, command argument, test log, or evidence file.
- `dev:backend` intentionally remains unavailable until a later runtime feature.
- Do not move `src`, `src-tauri`, Apple, or Android generated projects in this issue.
- T037/T038 indicate that ownership was transferred, not that device trials
  passed. Only close Issue #23 with actual physical-device evidence.
