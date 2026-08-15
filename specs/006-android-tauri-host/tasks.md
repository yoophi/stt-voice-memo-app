# Tasks: Minimal Android Tauri Host

**Input**: Design documents from `/specs/006-android-tauri-host/`

**Tests**: Required by the specification and constitution. Contract and mutation
tests are written before implementation; physical-device execution remains Issue
#23 and is never inferred from automated results.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with other tasks in the same phase
- **[US1]**, **[US2]**, **[US3]**: User story ownership
- Every task names an exact repository path or command

## Phase 1: Setup

- [x] T001 Confirm branch `024-android-tauri-host`, active feature path, and preserve untracked user files with `git status --short`
- [x] T002 [P] Record the locked Tauri/Gradle/SDK/NDK baseline in `specs/006-android-tauri-host/research.md`
- [x] T003 [P] Update current feature context in `AGENTS.md`
- [x] T004 Initialize the host once from the repository root with `pnpm tauri android init --ci`

---

## Phase 2: Foundational Contracts and Tests

**Purpose**: Establish fail-closed tests before pruning or accepting generated code.

- [x] T005 Add failing Android host presence, identity, API-floor, and tracked-file tests to `scripts/workspace/workspace-contract.test.mjs`
- [x] T006 [P] Add failing permission, feature, launcher, and component allowlist temporary mutation fixtures in `scripts/workspace/workspace-contract.test.mjs`
- [x] T007 Add failing unavailable/partial/invalid/verified state tests for `scripts/workspace/check-mobile-paths.mjs`
- [x] T008 [P] Add failing Kotlin activity and forbidden-resource tests to `scripts/workspace/workspace-contract.test.mjs`
- [x] T009 Add failing root command contract tests for `build:android` and `validate:android-host` in `scripts/workspace/workspace-contract.test.mjs`

**Checkpoint**: Tests fail because the generated host still contains unowned capabilities and validation is incomplete.

---

## Phase 3: User Story 1 — Build from a clean checkout (Priority: P1) 🎯 MVP

**Goal**: Track one stable API 24+ host and build an installable bundled APK from the repository root.

**Independent Test**: A tracked-only checkout runs `pnpm validate:android-host` and `pnpm build:android` without initialization.

- [x] T010 [US1] Normalize generated Gradle identity, API 24 floor, SDK configuration, Rust root, and wrapper inputs under `src-tauri/gen/android/`
- [x] T011 [US1] Preserve only required generated host source and launcher resources under `src-tauri/gen/android/app/`
- [x] T012 [US1] Restrict host-local ignore rules to build, IDE, SDK-path, generated, and signing state in `src-tauri/gen/android/.gitignore` and `src-tauri/gen/android/app/.gitignore`
- [x] T013 [US1] Add stable root `build:android` and `validate:android-host` scripts to `package.json`
- [x] T014 [US1] Implement complete-path, identity, min-SDK, and project-location validation in `scripts/workspace/check-mobile-paths.mjs`
- [x] T015 [US1] Update the existing workspace test from unavailable to verified Android expectations in `scripts/workspace/workspace-contract.test.mjs`
- [x] T016 [US1] Run `pnpm test:workspace` and make the US1 contract tests pass

**Checkpoint**: The tracked source host is independently valid and root commands are stable.

---

## Phase 4: User Story 2 — Minimum capability host (Priority: P2)

**Goal**: Accept only the current foreground touch launcher and reject all unowned capabilities.

**Independent Test**: Each mutation fixture fails with a stable category while the real host passes without permission or secret output.

- [x] T017 [US2] Replace the source manifest with the exact touchscreen/MainActivity allowlist in `src-tauri/gen/android/app/src/main/AndroidManifest.xml`
- [x] T018 [US2] Reduce `MainActivity` to a minimal `TauriActivity` subclass in `src-tauri/gen/android/app/src/main/java/com/yoophi/sttvoicememo/MainActivity.kt`
- [x] T019 [US2] Remove Leanback, FileProvider, file-path, unused layout/theme/color, and explicit edge-to-edge inputs under `src-tauri/gen/android/app/src/main/`
- [x] T020 [US2] Remove dependencies used only by deleted generated behavior from `src-tauri/gen/android/app/build.gradle.kts`
- [x] T021 [US2] Implement semantic source-manifest and activity allowlist validation in `scripts/workspace/check-mobile-paths.mjs`
- [x] T022 [US2] Make all permission/component/partial-host mutation tests pass in `scripts/workspace/workspace-contract.test.mjs`
- [x] T023 [US2] Extend built-client secret validation to cover Android native resources through the existing `pnpm verify:client-secret-boundary` workflow

**Checkpoint**: The app-owned host has no permission and one explicit launcher; mutations fail closed.

---

## Phase 5: User Story 3 — Build and physical handoff (Priority: P3)

**Goal**: Produce automated APK evidence and a content-safe, executable Issue #23 handoff.

**Independent Test**: An ARM64 APK builds, its merged manifest is inspected, and physical rows remain explicitly `Not run`.

- [x] T024 [US3] Add Android toolchain preflight with stable unavailable/invalid codes in `scripts/workspace/check-android-toolchain.mjs`
- [x] T025 [US3] Add packaged APK identity, SDK, permission, launcher, and merged-component validation in `scripts/workspace/check-android-apk.mjs`
- [x] T026 [P] [US3] Add contract tests for Android toolchain and APK inspection adapters to `scripts/workspace/workspace-contract.test.mjs`
- [x] T027 [US3] Build the ARM64 debug APK with `pnpm build:android` and validate it with `scripts/workspace/check-android-apk.mjs`
- [x] T028 [US3] Document tracking, regeneration, preflight, build, and artifact inspection in `docs/android-tauri-host.md` and `README.md`
- [x] T029 [US3] Create content-safe automated/physical evidence rows in `tests/device/android-tauri-host.md`, recording automated results only
- [x] T030 [US3] Verify Issue #23 can start from the merged revision without native generation or source edits and link the handoff in `tests/device/android-tauri-host.md`

**Checkpoint**: Automated evidence is complete; physical acceptance is visibly owned by Issue #23.

---

## Phase 6: Polish and Cross-Cutting Validation

- [x] T031 [P] Update formatting coverage for `specs/006-android-tauri-host`, Android docs, and device evidence in `package.json`
- [x] T032 [P] Update CI/path-selection documentation and contract coverage for Android host changes in `.github/workflows/validate.yml` and `scripts/workspace/workspace-contract.test.mjs`
- [x] T033 Run `pnpm validate:android-host`, `pnpm test:workspace`, and `pnpm verify:client-secret-boundary`
- [x] T034 Run `pnpm validate:mobile` and confirm frontend, Rust, Swift, iOS path, and secret checks remain passing
- [x] T035 Run `pnpm format:check` and inspect `git diff --check` plus `git status --short` without staging `.wtp.yml`
- [ ] T036 [#23] Execute physical API 24+ Android install, cold-launch, no-permission, and sanitized unsupported-plugin trials in follow-up Issue #23 after merge

## Dependencies and Execution Order

```text
Phase 1
  └─ Phase 2 tests
       └─ US1 stable host/build
            └─ US2 capability pruning
                 └─ US3 packaged proof/handoff
                      └─ Polish and validation
                           └─ #23 physical execution after merge
```

- T004 must precede tests that inspect the real generated baseline.
- T005–T009 must fail before T010–T025 implement their contracts.
- US1 is required before capability and packaged APK checks can run.
- US2 source allowlist is required before the APK may be accepted in US3.
- T036 is intentionally outside Issue #24 implementation completion and remains unchecked.

## Parallel Opportunities

- T002 and T003 can proceed independently.
- Fixture creation T006 and activity/resource tests T008 can proceed after generation while core path tests are written.
- Documentation/evidence T028–T029 can proceed alongside adapter tests T026 after command names are fixed.
- Formatting coverage T031 and CI/path-selection coverage T032 touch different files.

## Implementation Strategy

1. Generate once, immediately capture RED tests for the unsafe template baseline.
2. Deliver US1 so clean-checkout source discovery and root build are stable.
3. Prune and validate source capabilities for US2.
4. Build and inspect the actual merged APK for US3.
5. Preserve physical validation as an explicit unchecked Issue #23 gate.
