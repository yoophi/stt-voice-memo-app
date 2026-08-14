---

description: "Task list template for feature implementation"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Domain rules, port contracts, recording/transcription/persistence
boundaries, and security-sensitive behavior require tests. Recording or lifecycle
changes also require physical iOS and Android device-validation tasks. Add other
tests according to feature risk and the specification.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **React FSD**: `src/app`, `src/pages`, `src/widgets`, `src/features`,
  `src/entities`, and `src/shared`
- **Rust Hexagonal**: `src-tauri/src/domain`, `ports`, `application`, `inbound`,
  and `infrastructure`
- **Platform adapters**: Tauri-generated iOS/Android projects and plugin paths
  selected by `plan.md`
- **Tests**: colocated unit tests plus `tests/contract`, `tests/integration`, and
  device-validation evidence/tasks selected by `plan.md`

<!--
  ============================================================================
  IMPORTANT: The tasks below are SAMPLE TASKS for illustration purposes only.

  The /speckit-tasks command MUST replace these with actual tasks based on:
  - User stories from spec.md (with their priorities P1, P2, P3...)
  - Feature requirements from plan.md
  - Entities from data-model.md
  - Endpoints from contracts/

  Tasks MUST be organized by user story so each story can be:
  - Implemented independently
  - Tested independently
  - Delivered as an MVP increment

  DO NOT keep these sample tasks in the generated tasks.md file.
  ============================================================================
-->

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Create project structure per implementation plan
- [ ] T002 Initialize [language] project with [framework] dependencies
- [ ] T003 [P] Configure linting and formatting tools

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

Examples of foundational tasks (adjust based on your project):

- [ ] T004 Setup local memo persistence and migrations framework
- [ ] T005 [P] Define recorder and transcription ports
- [ ] T006 [P] Setup backend API client boundary and authentication flow
- [ ] T007 Create base recording, transcription, and memo domain entities
- [ ] T008 Configure error handling and logging infrastructure
- [ ] T009 Setup environment configuration management
- [ ] T010 Define backend-only credential handling and audio/transcript retention boundaries
- [ ] T011 Establish iOS and Android device-validation procedure for affected workflows

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - [Title] (Priority: P1) 🎯 MVP

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T012 [P] [US1] Contract test for [port/API] in [exact test path]
- [ ] T013 [P] [US1] Integration test for [user journey] in [exact test path]
- [ ] T014 [US1] Verify relevant permission, interruption, and recovery flow on physical iOS and Android devices

### Implementation for User Story 1

- [ ] T015 [P] [US1] Create [domain entity] in [exact domain/entity path]
- [ ] T016 [P] [US1] Define [port or frontend entity] in [exact path]
- [ ] T017 [US1] Implement [use case/feature] in [exact path] (depends on T015, T016)
- [ ] T018 [US1] Implement [adapter/UI] in [exact path]
- [ ] T019 [US1] Add validation, recovery, and privacy-safe diagnostics

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - [Title] (Priority: P2)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 2

- [ ] T020 [P] [US2] Contract test for [port/API] in [exact test path]
- [ ] T021 [P] [US2] Integration test for [user journey] in [exact test path]
- [ ] T022 [US2] Verify applicable mobile lifecycle or recovery flow on physical iOS and Android devices

### Implementation for User Story 2

- [ ] T023 [P] [US2] Create [domain/frontend entity] in [exact path]
- [ ] T024 [US2] Implement [use case/feature] in [exact path]
- [ ] T025 [US2] Implement [adapter/UI] in [exact path]
- [ ] T026 [US2] Integrate with User Story 1 components (if needed)

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - [Title] (Priority: P3)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 3

- [ ] T027 [P] [US3] Contract test for [port/API] in [exact test path]
- [ ] T028 [P] [US3] Integration test for [user journey] in [exact test path]
- [ ] T029 [US3] Verify applicable mobile lifecycle or recovery flow on physical iOS and Android devices

### Implementation for User Story 3

- [ ] T030 [P] [US3] Create [domain/frontend entity] in [exact path]
- [ ] T031 [US3] Implement [use case/feature] in [exact path]
- [ ] T032 [US3] Implement [adapter/UI] in [exact path]

**Checkpoint**: All user stories should now be independently functional

---

[Add more user story phases as needed, following the same pattern]

---

## Phase N: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] TXXX [P] Documentation updates in docs/
- [ ] TXXX Code cleanup and refactoring
- [ ] TXXX Performance optimization across all stories
- [ ] TXXX [P] Additional risk-based unit tests in [exact test paths]
- [ ] TXXX Security hardening
- [ ] TXXX Verify audio/transcript retention, deletion, and privacy-safe logging
- [ ] TXXX Run applicable physical iOS and Android regression scenarios
- [ ] TXXX Run quickstart.md validation

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - May integrate with US1 but should be independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - May integrate with US1/US2 but should be independently testable

### Within Each User Story

- Required automated tests MUST be written and fail before implementation
- Models before services
- Services before endpoints
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel
- Models within a story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Contract test for [port/API] in [exact test path]"
Task: "Integration test for [user journey] in [exact test path]"

# Launch all models for User Story 1 together:
Task: "Create [Entity1] model in src/models/[entity1].py"
Task: "Create [Entity2] model in src/models/[entity2].py"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
   - Developer C: User Story 3
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
