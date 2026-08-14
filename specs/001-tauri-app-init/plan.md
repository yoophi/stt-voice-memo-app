# Implementation Plan: Mobile Tauri App Foundation

**Branch**: `main` | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-tauri-app-init/spec.md`

## Summary

비어 있지 않은 현재 저장소에 React + TypeScript + Vite 프런트엔드를 만들고 Tauri 2를
수동 초기화한 뒤, iOS와 Android 생성 프로젝트를 함께 구성한다. React는 Feature-Sliced
Design 계층과 단일 app provider 조립 지점을, Rust는 Hexagonal Architecture의 순수
내부 계층과 외부 adapter 경계를 먼저 확립한다. 초기 화면은 `STT Voice Memo`와 기반
준비 상태만 표시하며 권한, 녹음, 네트워크, 저장 기능은 포함하지 않는다. pnpm lockfile,
통합 검증 명령, 플랫폼별 doctor 진단, 실제 iOS/Android 기기 smoke test 절차로 재현성과
모바일 우선 완료 조건을 검증한다.

## Technical Context

**Language/Version**: TypeScript 5.x, React 19.x, Node.js 22.22+, Rust stable 1.95
(edition 2024), Swift/Kotlin은 Tauri 생성 프로젝트에 한정

**Primary Dependencies**: Tauri 2.x (`tauri`, `tauri-build`, `@tauri-apps/api`,
`@tauri-apps/cli`), Vite, Tailwind CSS 4, shadcn/ui, TanStack Query 5, Zustand 5

**Storage**: N/A — 이 기능은 오디오, transcript, memo, 인증 또는 사용자 설정을
생성하거나 저장하지 않음

**Testing**: Vitest + React Testing Library, `cargo test`, ESLint, Prettier,
TypeScript project build, 플랫폼 doctor 자동 테스트, 실제 iOS/Android 기기 smoke test

**Target Platform**: iOS 15+ 및 Android 7/API 24+; Android는 최신 보안 업데이트가
적용된 Chromium 기반 System WebView 사용; macOS 개발 호스트에서 실제 기기 우선 검증

**Project Type**: 단일 저장소의 Tauri mobile application (React webview + Rust core +
Tauri 생성 Swift/Kotlin host projects)

**Performance Goals**: 작은 휴대전화에서 초기 shell이 60 fps 상호작용을 유지하고,
준비된 환경에서 cold launch 후 2초 내 초기 콘텐츠를 표시하며, SDK 다운로드를 제외한
새 checkout 첫 실행을 15분 내 완료

**Constraints**: 모바일 우선, foreground shell만 제공, 런타임 민감 권한 0개,
네트워크 요청 0개, 비밀 값 0개, safe-area/회전 대응, 포트 1420 고정 및 충돌 시 명확한
오류, 서버 상태를 Zustand에 복제하지 않음

**Scale/Scope**: 초기 화면 1개, app composition root 1개, FSD 6계층, Rust hexagonal
5경계, iOS/Android 2개 대상, 영속 엔터티 및 외부 API 0개

## Constitution Check

*GATE: Phase 0 이전 및 Phase 1 설계 후 재검증 — 모두 PASS.*

- **Mobile first — PASS**: iOS 15+와 Android API 24+를 명시하고 safe-area, 회전,
  foreground/background 복귀, touch viewport, 실제 기기 cold launch를 계약과 quickstart에
  포함한다. 초기 shell에는 권한과 오디오 세션이 없으며 desktop 완료 조건은 제외한다.
- **Hexagonal Rust — PASS**: `domain`과 `application`은 Tauri/OS에 의존하지 않고,
  outbound 계약은 `ports`, Tauri 진입점은 `inbound`, 플랫폼 및 향후 외부 구현은
  `infrastructure`에 둔다. 현재 유스케이스가 없으므로 허위 command나 port를 만들지 않는다.
- **Feature-Sliced React — PASS**: `app → pages → widgets → features → entities → shared`
  import 방향을 고정한다. TanStack Query provider는 `app/providers`에서 조립하지만 서버
  query를 만들지 않고, Zustand는 실제 클라이언트 상태가 생길 때 feature/entity가 소유하며
  초기 빈 store를 만들지 않는다.
- **Secure transcription — PASS (범위 외 확인)**: 이 기능은 backend/API/오디오/transcript가
  없고 provider credential이나 `.env` 비밀을 요구하지 않는다. Tauri capability는 shell에
  필요한 최소 core 권한만 가지며 마이크 등 민감 권한을 선언하지 않는다.
- **Resilient voice flow — PASS (범위 외 확인)**: recording-to-memo 상태는 이 기능에서
  만들지 않는다. 대신 시작 실패, 누락 도구, 포트 충돌, lifecycle 복귀를 검증하고 실제
  iOS/Android 기기에서 각각 5회 cold launch한다. 음성 흐름은 별도 명세 없이는 추가하지 않는다.

**Post-design re-check**: `data-model.md`에 사용자 콘텐츠가 없음을 명시했고,
`contracts/app-shell.md`와 `contracts/developer-commands.md`가 권한·경계·기기 검증을
구체화한다. 설계 후에도 위 다섯 gate는 모두 PASS이며 예외 승인은 필요하지 않다.

## Project Structure

### Documentation (this feature)

```text
specs/001-tauri-app-init/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── app-shell.md
│   └── developer-commands.md
└── tasks.md                    # /speckit-tasks에서 생성
```

### Source Code (repository root)

```text
src/
├── app/
│   ├── providers/
│   │   └── query-client-provider.tsx
│   ├── styles/
│   │   └── globals.css
│   └── App.tsx
├── pages/
│   └── home/
│       ├── index.ts
│       └── ui/home-page.tsx
├── widgets/
│   └── foundation-status/
│       ├── index.ts
│       └── ui/foundation-status.tsx
├── features/
│   └── README.md               # 사용자 행동이 생길 때 slice 추가
├── entities/
│   └── README.md               # 도메인 표현이 생길 때 slice 추가
├── shared/
│   ├── config/app-identity.ts
│   ├── lib/utils.ts
│   └── ui/card.tsx
├── main.tsx
└── vite-env.d.ts

src-tauri/
├── capabilities/
│   └── default.json
├── gen/
│   ├── android/                # Tauri가 생성한 Android/Kotlin host project
│   └── apple/                  # Tauri가 생성한 iOS/Swift host project
├── icons/
├── src/
│   ├── domain/mod.rs
│   ├── application/mod.rs
│   ├── ports/mod.rs
│   ├── inbound/mod.rs
│   ├── infrastructure/mod.rs
│   ├── lib.rs                  # adapter 조립 및 mobile library entry
│   └── main.rs                 # desktop-compatible thin entry; 완료 조건은 아님
├── Cargo.toml
├── Cargo.lock
├── build.rs
└── tauri.conf.json

scripts/
├── mobile-doctor.mjs
└── mobile-doctor.test.mjs

tests/
└── device/
    └── mobile-shell-smoke.md
```

**Structure Decision**: 단일 제품이므로 현 단계에서 workspace/monorepo를 만들지 않는다.
React의 공개 import는 각 slice의 `index.ts`를 통하고 `shared`만 상위 계층을 모른다.
Rust의 `lib.rs`는 composition root일 뿐 규칙을 소유하지 않으며, `domain/application/ports`
내부를 Tauri에서 분리한다. 아직 비즈니스 동작이 없는 계층은 경계 설명만 두고 가짜 모델,
command, port, Zustand store를 만들지 않는다. Tauri가 생성한 두 mobile host project는
재현 가능한 실제 기기 실행을 위해 소스 관리하되 build output과 로컬 signing 값은 제외한다.

## Implementation Strategy

1. React + TypeScript Vite 기반과 pnpm lockfile을 만든 후 Tauri CLI를 기존 저장소에
   수동 초기화한다. 앱 식별자는 `com.yoophi.sttvoicememo`, 표시명은 `STT Voice Memo`로
   일관되게 설정한다.
2. Vite에 Tailwind CSS 4, `@/*` alias, `TAURI_DEV_HOST`, strict port 1420 및
   `src-tauri` watch 제외를 구성하고 shadcn/ui의 최소 `Card`만 생성한다.
3. React FSD와 Rust hexagonal 경계를 만든 뒤, 실제 초기 화면과 query provider 조립만
   연결한다. 민감 capability, plugin, Zustand store, network query는 추가하지 않는다.
4. `tauri ios init`과 `tauri android init`으로 host projects를 생성하고 플랫폼별 doctor와
   실행 script를 제공한다. 재생성은 같은 CLI init 명령을 사용한다.
5. format/lint/type/build/unit/Rust 검증 후 실제 iOS와 Android 기기에서 viewport, 회전,
   lifecycle, 무권한 시작과 5회 cold launch를 기록한다.

## Complexity Tracking

Constitution 위반이나 정당화가 필요한 추가 복잡성은 없다.
