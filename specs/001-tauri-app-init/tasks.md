# Tasks: Mobile Tauri App Foundation

**Input**: Design documents from `/specs/001-tauri-app-init/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`,
`contracts/`, `quickstart.md`

**Tests**: 명세가 자동 테스트, lifecycle 검증 및 실제 iOS/Android 기기 검증을 완료 조건으로
요구하므로 각 사용자 스토리에 해당 검증 작업을 포함한다.

**Organization**: 작업은 사용자 스토리별로 구성하며, 각 스토리는 공통 foundation 이후
독립적으로 구현하고 검증할 수 있다.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: 미완료 작업과 파일 충돌 없이 병렬 실행 가능
- **[Story]**: 사용자 스토리 추적 라벨 (`US1`, `US2`, `US3`)
- 모든 작업 설명에는 생성하거나 수정할 정확한 파일 경로를 포함

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 현재 문서 중심 저장소를 재현 가능한 React/Tauri 애플리케이션으로 초기화

- [X] T001 Create the pnpm 11 React 19/TypeScript 5/Vite dependency manifest and reproducible lockfile with Tauri 2, Tailwind CSS 4, shadcn/ui, TanStack Query 5, Zustand 5, Vitest, Testing Library, ESLint, and Prettier in `package.json` and `pnpm-lock.yaml`
- [X] T002 [P] Configure the Vite React entry, TypeScript project references, `@/*` alias, Tailwind plugin, strict port 1420, `TAURI_DEV_HOST` host/HMR handling, and `src-tauri` watch exclusion in `index.html`, `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`, and `vite.config.ts`
- [X] T003 [P] Configure ESLint, Prettier, Vitest jsdom, and Testing Library cleanup/matchers in `eslint.config.js`, `.prettierrc.json`, `.prettierignore`, `vitest.config.ts`, and `src/test/setup.ts`
- [X] T004 Initialize the existing repository as a Tauri 2 Rust edition-2024 application with mobile-compatible library and thin desktop entry points in `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/build.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, and `src-tauri/tauri.conf.json`
- [X] T005 [P] Exclude dependency caches, build products, local signing state, environment secrets, and IDE artifacts while preserving `pnpm-lock.yaml`, `src-tauri/Cargo.lock`, and generated mobile source projects in `.gitignore`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 모든 사용자 스토리가 공유하는 UI, 상태 조립 및 최소 권한 기반 확립

**⚠️ CRITICAL**: 이 phase가 완료되기 전에는 사용자 스토리 구현을 시작하지 않는다.

- [X] T006 [P] Initialize shadcn/ui for Vite with Tailwind CSS 4 and add only the reusable Card primitive and class-name utility in `components.json`, `src/app/styles/globals.css`, `src/shared/ui/card.tsx`, and `src/shared/lib/utils.ts`
- [X] T007 [P] Create the single TanStack Query composition provider without queries, mutations, persistence, devtools, or a Zustand store in `src/app/providers/query-client-provider.tsx`
- [X] T008 [P] Configure the `STT Voice Memo` identity, `com.yoophi.sttvoicememo` identifier, iOS 15 minimum, Android API 24 minimum, frontend URLs, and least-privilege shell capability with no sensitive plugins or permissions in `src-tauri/tauri.conf.json` and `src-tauri/capabilities/default.json`

**Checkpoint**: React/Tauri가 빌드 가능한 공통 기반과 최소 권한 조립점이 준비되어 모든
사용자 스토리를 시작할 수 있다.

---

## Phase 3: User Story 1 - Run the Mobile App Shell (Priority: P1) 🎯 MVP

**Goal**: 새 checkout에서 실제 iOS와 Android 기기에 앱을 실행하여 제품 정체성과 준비
상태를 보여주는 무권한 mobile-first shell을 확인한다.

**Independent Test**: 준비된 환경에서 raw Tauri mobile 명령으로 양 플랫폼 실제 기기에
설치한 뒤 heading/상태 문구, safe-area와 회전, background/foreground 복귀, 민감 권한
prompt 부재 및 각 5회 cold launch 성공을 확인한다.

### Tests for User Story 1

- [X] T009 [US1] Write failing UI contract tests for the product heading, honest foundation status, absence of recording/transcription controls, single QueryClientProvider composition, and zero initial Tauri/network side effects in `src/pages/home/ui/home-page.test.tsx`
- [X] T010 [P] [US1] Create the physical-device evidence template with iOS/Android device metadata, portrait/landscape, lifecycle, five cold launches, and unexpected permission prompt fields in `tests/device/mobile-shell-smoke.md`

### Implementation for User Story 1

- [X] T011 [P] [US1] Define the non-persisted product name, bundle identifier, version, and development port constants without environment secrets in `src/shared/config/app-identity.ts`
- [X] T012 [P] [US1] Implement the presentational foundation status widget with shadcn Card, semantic status text, and no interactive fake controls in `src/widgets/foundation-status/ui/foundation-status.tsx` and `src/widgets/foundation-status/index.ts`
- [X] T013 [US1] Compose the Home page from App and the query provider, wire the React entry, and implement `100svh`, safe-area, 320×568, portrait, and landscape styling in `src/pages/home/ui/home-page.tsx`, `src/pages/home/index.ts`, `src/app/App.tsx`, `src/main.tsx`, and `src/app/styles/globals.css`
- [X] T014 [US1] Generate and review the iOS Swift host project with `pnpm tauri ios init`, ensuring no microphone or other sensitive usage descriptions are introduced in `src-tauri/gen/apple/`
- [ ] T015 [US1] Generate and review the Android Kotlin host project with `pnpm tauri android init`, ensuring no microphone or other sensitive permissions are introduced in `src-tauri/gen/android/`
- [ ] T016 [US1] Run `pnpm tauri ios dev` on a physical iOS 15+ device, execute every app-shell acceptance scenario including five cold launches, and record passing evidence in the iOS section of `tests/device/mobile-shell-smoke.md`
- [ ] T017 [US1] Run `pnpm tauri android dev` on a physical Android API 24+ device with an up-to-date System WebView, execute every app-shell acceptance scenario including five cold launches, and record passing evidence in the Android section of `tests/device/mobile-shell-smoke.md`

**Checkpoint**: User Story 1은 실제 iOS/Android 기기에서 독립 실행·검증 가능한 MVP다.

---

## Phase 4: User Story 2 - Extend a Predictable Project Structure (Priority: P2)

**Goal**: 새 개발자가 화면, 사용자 행동, 도메인 표현, Rust 규칙 및 플랫폼 adapter의 위치와
의존 방향을 구조와 문서만으로 식별한다.

**Independent Test**: 프로젝트를 처음 보는 개발자가 네 가지 예시 변경 위치와 상태 소유자를
5분 내 올바르게 답하고, 자동 architecture contract가 FSD/Hexagonal 경계와 빈 Zustand store
부재를 검증한다.

### Tests for User Story 2

- [ ] T018 [P] [US2] Write a failing filesystem/import architecture contract for all six FSD layers, public slice entry points, forbidden lower-to-higher imports, one query composition root, and absence of empty Zustand stores in `tests/architecture/react-fsd.test.ts`
- [ ] T019 [P] [US2] Write a failing Rust architecture contract that requires the five hexagonal modules and rejects Tauri, OS, filesystem, database, HTTP, and OpenAI dependencies from domain/application source in `src-tauri/tests/architecture.rs`

### Implementation for User Story 2

- [ ] T020 [P] [US2] Add FSD boundary guidance and tracked placeholders for future user-action and domain-expression slices without creating dummy state or behavior in `src/README.md`, `src/features/README.md`, and `src/entities/README.md`
- [ ] T021 [P] [US2] Establish documented Rust domain, application, ports, inbound, and infrastructure module boundaries while keeping `lib.rs` as the composition root and avoiding fake entities, commands, and traits in `src-tauri/src/domain/mod.rs`, `src-tauri/src/application/mod.rs`, `src-tauri/src/ports/mod.rs`, `src-tauri/src/inbound/mod.rs`, `src-tauri/src/infrastructure/mod.rs`, and `src-tauri/src/lib.rs`
- [ ] T022 [US2] Document the FSD import direction, Hexagonal dependency direction, TanStack Query/Zustand ownership rule, Tauri composition boundary, and concrete placement examples in `docs/architecture.md`
- [ ] T023 [US2] Conduct the five-minute newcomer placement exercise for page, user action, domain rule, and platform adapter changes and record answers, elapsed time, and corrections in `tests/architecture/boundary-review.md`

**Checkpoint**: User Story 2의 구조 계약과 newcomer exercise를 US1 UI 동작과 독립적으로
검증할 수 있다.

---

## Phase 5: User Story 3 - Validate the Foundation Consistently (Priority: P3)

**Goal**: 개발자가 한 명령으로 전체 foundation을 검사하고, 플랫폼 도구 누락이나 설치 중단을
삭제 작업 없이 진단·복구한다.

**Independent Test**: 정상 환경에서 전체 검증이 성공하고, stubbed Java/Xcode/SDK 누락 및
한쪽 플랫폼만 준비된 환경에서 doctor가 stable check ID와 다음 조치를 출력하며 올바른 exit
code를 반환하는지 확인한다.

### Tests for User Story 3

- [ ] T024 [P] [US3] Write failing doctor tests for common/iOS/Android target filtering, PASS/WARN/FAIL/SKIP output, missing Java/Xcode/SDK/Rust target/device actions, secret-safe diagnostics, and platform-independent exit status in `scripts/mobile-doctor.test.mjs`
- [ ] T025 [P] [US3] Write a failing developer-command contract test for pinned package manager and lockfiles, strict-port config, validation scripts, raw mobile aliases, and repeatable init/recovery commands in `tests/contract/developer-commands.test.ts`

### Implementation for User Story 3

- [ ] T026 [US3] Implement the side-effect-free common/iOS/Android prerequisite inspector with injectable command/environment probes, stable check IDs, official next-action links, platform-isolated results, and redacted values in `scripts/mobile-doctor.mjs`
- [ ] T027 [US3] Add fail-fast `format:check`, `lint`, `typecheck`, `test`, `check`, `doctor`, `doctor:ios`, `doctor:android`, and mobile init/dev aliases matching the developer command contract in `package.json`
- [ ] T028 [US3] Document clean checkout setup, interrupted-install retry, platform-specific prerequisites, doctor output, generated-project recovery, port conflicts, physical-device commands, and the absence of client secrets in `README.md` and `docs/development.md`
- [ ] T029 [US3] Run the normal, missing-Java, missing-Xcode, and one-platform-ready doctor scenarios and record commands, exit codes, stable check IDs, redaction checks, and next actions in `tests/integration/mobile-doctor-validation.md`
- [ ] T030 [US3] Validate repeated `pnpm install`, frozen-lockfile installation, and missing generated mobile project recovery in a disposable checkout without deleting source or caches, recording results in `tests/integration/recovery-validation.md`
- [ ] T031 [US3] Run the complete `pnpm check` pipeline and record each format, lint, typecheck, Vitest, Cargo test, clippy, and production build result in `tests/integration/foundation-check.md`

**Checkpoint**: 모든 사용자 스토리가 구현되어 정상·누락 도구·복구 환경에서 독립 검증 가능하다.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: 전체 feature의 재현성, 보안 및 문서 정합성을 최종 검증

- [ ] T032 [P] Audit the repository for credential patterns, `.env` secrets, unexpected Tauri plugins/capabilities, iOS usage descriptions, Android permissions, network calls, and recording/transcription code, recording findings in `tests/security/foundation-audit.md`
- [ ] T033 [P] Measure a fresh checkout from dependency-ready state through first mobile launch, excluding SDK downloads, and record whether the 15-minute target is met in `tests/integration/clean-checkout-validation.md`
- [ ] T034 Re-run every scenario in `specs/001-tauri-app-init/quickstart.md`, resolve documentation/command drift in that file, and append final completion evidence to `tests/integration/quickstart-validation.md`
- [ ] T035 Run Prettier and Rust formatting, verify all generated source and documentation are tracked while caches/signing data remain ignored, and record the final reviewed file set in `tests/integration/final-repository-review.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: 즉시 시작 가능. T001과 T004는 순서대로 진행하며 T002, T003, T005는
  파일 충돌 없이 병렬 진행할 수 있다.
- **Foundational (Phase 2)**: Phase 1 완료에 의존하며 모든 사용자 스토리를 차단한다.
- **User Story 1 (Phase 3)**: Phase 2 이후 시작. T009 테스트를 먼저 실패시킨 뒤 T011–T013을
  구현하고, T014/T015 생성 후 T016/T017 실제 기기 검증을 수행한다.
- **User Story 2 (Phase 4)**: Phase 2 이후 US1과 독립 시작 가능. T018/T019 테스트를 먼저
  실패시킨 뒤 각 경계를 구현하고 T023으로 검증한다.
- **User Story 3 (Phase 5)**: Phase 2 이후 시작 가능. T024/T025 테스트를 먼저 실패시킨 뒤
  T026/T027을 구현하고 T029–T031로 정상·실패·복구 흐름을 검증한다.
- **Polish (Phase 6)**: 릴리스하려는 US1–US3와 모든 실제 기기 검증 완료에 의존한다.

### User Story Dependency Graph

```text
Setup (T001-T005)
        |
Foundational (T006-T008)
        |
        +----------------+----------------+
        |                |                |
   US1 / MVP         US2 / Structure   US3 / Validation
   T009-T017         T018-T023         T024-T031
        |                |                |
        +----------------+----------------+
                         |
                  Polish (T032-T035)
```

- **US1 (P1)**: 다른 스토리에 의존하지 않으며 Phase 2 이후 독립 구현·검증 가능
- **US2 (P2)**: US1에 의존하지 않지만 동일 repository skeleton을 사용
- **US3 (P3)**: US1/US2 구현에 의존하지 않지만 최종 `pnpm check`와 quickstart 증거는 선택된
  모든 스토리가 완료된 뒤 갱신

### Parallel Opportunities

- Phase 1의 T002, T003, T005
- Phase 2의 T006, T007, T008
- US1의 T010–T012 중 서로 다른 파일을 다루는 작업
- US2의 T018–T019와 T020–T021
- US3의 T024–T025
- Phase 2 후 US1, US2, US3 자체를 서로 병렬 진행
- Polish의 T032–T033

---

## Parallel Examples

### User Story 1

```text
Task T011: app identity in src/shared/config/app-identity.ts
Task T012: foundation widget in src/widgets/foundation-status/

```

### User Story 2

```text
Task T018: React architecture test in tests/architecture/react-fsd.test.ts
Task T019: Rust architecture test in src-tauri/tests/architecture.rs

Task T020: React FSD boundaries in src/
Task T021: Rust Hexagonal boundaries in src-tauri/src/
```

### User Story 3

```text
Task T024: doctor behavior tests in scripts/mobile-doctor.test.mjs
Task T025: command contract tests in tests/contract/developer-commands.test.ts
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. T001–T005로 React/Tauri 저장소를 초기화한다.
2. T006–T008로 공통 UI, provider, 최소 권한 기반을 완성한다.
3. T009/T010 검증 계약을 먼저 준비한다.
4. T011–T015로 shell과 mobile host projects를 구현한다.
5. T016/T017로 실제 iOS/Android 기기에서 독립 검증한다.
6. **STOP AND VALIDATE**: 양 플랫폼 5회 cold launch와 권한 prompt 0회를 확인한 뒤 MVP로
   시연한다.

### Incremental Delivery

1. Setup + Foundational → 빌드 가능한 최소 기반
2. US1 → 실제 모바일 shell MVP
3. US2 → 확장 가능한 FSD/Hexagonal 경계
4. US3 → 반복 가능한 자동 검증과 도구 진단
5. Polish → clean checkout, 보안, 문서 정합성 완료

### Suggested Single-Agent Order

파일 충돌과 재작업을 줄이기 위해 `T001 → T004 → T002/T003/T005 → T006–T008 → US1 →
US2 → US3 → Polish` 순으로 실행한다. `[P]`는 독립 작업자나 별도 agent가 있을 때만 병렬화한다.

## Notes

- `[P]`는 서로 다른 파일을 수정하고 미완료 작업에 의존하지 않는 작업만 의미한다.
- story phase의 모든 작업은 `[US1]`, `[US2]`, `[US3]` 라벨을 가진다.
- 이 feature에서는 audio, transcript, memo, credential, backend API, microphone permission,
  background recording, realtime transcription 또는 desktop release task를 추가하지 않는다.
- 실제 기기와 signing 접근이 필요한 T016/T017은 자동화로 대체하지 않으며 완료 증거가 없으면
  US1을 완료 처리하지 않는다.
