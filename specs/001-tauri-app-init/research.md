# Phase 0 Research: Mobile Tauri App Foundation

## 1. 기존 저장소 초기화 방식

**Decision**: React + TypeScript Vite 프런트엔드를 현재 저장소 루트에 만든 뒤
`pnpm tauri init`으로 Tauri backend를 수동 초기화한다.

**Rationale**: 저장소에는 README, constitution, specs가 이미 존재한다. 공식 Tauri 문서는
기존 frontend에 CLI를 설치하고 `tauri init`을 실행하는 방식을 지원한다. 새 하위 디렉터리를
만드는 `create-tauri-app`보다 기존 문서를 보존하고 단일 앱 루트를 유지하기 쉽다.

**Alternatives considered**:

- `pnpm create tauri-app`: 깨끗한 새 디렉터리에는 적합하지만 현재 저장소의 기존 파일과
  합치는 별도 단계가 필요하다.
- monorepo의 `apps/mobile`: 현재 제품과 배포 단위가 하나뿐이라 불필요한 workspace 경계를
  만든다. 다중 앱 요구가 실제로 생길 때 별도 계획으로 전환한다.

**Sources**: [Tauri Create a Project](https://v2.tauri.app/start/create-project/)

## 2. 기술 버전과 의존성 기준

**Decision**: Tauri 2, React 19, TypeScript 5, Vite, Tailwind CSS 4, shadcn/ui,
TanStack Query 5, Zustand 5를 사용한다. 구현 시 선택한 호환 patch 버전은 `pnpm-lock.yaml`과
`Cargo.lock`으로 고정하고 `packageManager: pnpm@11.0.9`를 기록한다. Rust crate는 edition
2024를 사용한다.

**Rationale**: 이 머신에서 Node 22.22, pnpm 11.0.9, Rust 1.95가 확인되었다. 참고하는
Agentic Workspace의 최신 Tauri 앱도 Tailwind 4, Query 5, Zustand 5, Vitest 4 및 Rust
edition 2024 조합을 사용한다. major를 명시하고 lockfile을 커밋하면 최신 compatible patch를
사용하면서 checkout 재현성을 확보할 수 있다.

**Alternatives considered**:

- Handy의 React 18/TypeScript 5.6 조합 고정: 기능 패턴은 참고할 수 있지만 새 앱의 기반을
  오래된 frontend 조합에 고정할 이유가 없다.
- Tailwind CSS 3: Agentic Workspace의 구 앱에는 남아 있으나 최신 앱과 shadcn Vite 공식
  설정은 Tailwind 4 plugin 방식을 사용한다.
- 모든 버전을 floating `latest`로 유지: 최초 생성에는 편하지만 checkout 결과가 시간에
  따라 바뀐다.

**Sources**: [shadcn/ui Vite installation](https://ui.shadcn.com/docs/installation/vite),
[TanStack Query installation](https://tanstack.com/query/latest/docs/framework/react/installation)

## 3. 모바일 대상과 개발 서버

**Decision**: Tauri 기본 mobile bundle 기준인 Android API 24+와 iOS 14+ 중, TanStack
Query의 현재 브라우저 호환 범위를 만족하도록 제품 최소 대상을 iOS 15+로 올린다. Android는
API 24+에서 최신 업데이트가 적용된 System WebView를 전제로 한다. Vite는 port 1420을
strict하게 사용하고 `TAURI_DEV_HOST`가 있으면 host/HMR에 반영한다.

**Rationale**: Tauri 공식 config 기본값은 Android minSdk 24와 iOS minimumSystemVersion
14다. 그러나 TanStack Query 공식 호환 범위는 Safari/iOS 15 이상이다. Tauri는 Android에서
시스템 WebView를 번들하지 않으므로 실제 기기의 provider 버전을 점검해야 한다. 실제 iOS
기기는 CLI가 전달하는 `TAURI_DEV_HOST`를 Vite가 수신해야 안정적으로 개발 서버에 연결된다.

**Alternatives considered**:

- iOS 14 유지와 polyfill/transpile 추가: 초기 기반의 호환성 복잡도를 늘리고 검증 범위를
  키운다.
- Vite를 항상 `0.0.0.0`에 노출: 필요하지 않은 LAN 노출을 만든다.
- 동적 port 사용: Tauri `devUrl`과 HMR 설정을 복잡하게 하고 포트 충돌을 숨긴다.

**Sources**: [Tauri configuration defaults](https://v2.tauri.app/reference/config/),
[Tauri Vite checklist](https://v2.tauri.app/start/frontend/vite/),
[Tauri mobile development](https://v2.tauri.app/develop/),
[Tauri Webview Versions](https://v2.tauri.app/reference/webview-versions/),
[TanStack Query compatibility](https://tanstack.com/query/latest/docs/framework/react/installation)

## 4. 모바일 생성과 실행 명령

**Decision**: `pnpm tauri ios init`, `pnpm tauri android init`으로 host projects를 생성하고
`pnpm tauri ios dev` 및 `pnpm tauri android dev`로 실제 기기 실행을 표준화한다. package
scripts는 이 명령을 기억하기 쉬운 별칭으로 제공한다.

**Rationale**: Tauri CLI가 각 mobile target의 생성, 개발 실행 및 IDE 열기를 공식적으로
지원한다. 생성 프로젝트를 직접 복사하거나 수동 패치하는 것보다 같은 init 명령으로 복구하는
경로가 재현 가능하다.

**Alternatives considered**:

- Xcode/Android Studio project 수동 생성: Tauri 설정과 생성 템플릿의 일관성을 잃는다.
- simulator만 완료 기준으로 사용: constitution과 feature acceptance의 실제 기기 요구를
  만족하지 않는다.

**Sources**: [Tauri CLI mobile commands](https://v2.tauri.app/reference/cli/),
[Tauri mobile development](https://v2.tauri.app/develop/)

## 5. 플랫폼 사전 요구사항과 진단

**Decision**: 공통 `mobile-doctor.mjs`가 Node/pnpm/Rust와 플랫폼별 외부 도구, 환경 변수,
Rust targets, 연결된 device를 검사하고 각 실패에 설치 문서 링크와 다음 명령을 출력한다.
iOS 또는 Android 한쪽 실패가 다른 쪽 검사/개발을 막지 않도록 target argument를 지원한다.

**Rationale**: 현재 머신은 Xcode 27과 Rust/Node는 탐지되지만 `java`는 탐지되지 않는다.
이는 앱 소스 실패가 아니라 Android prerequisite 실패다. 사전 진단을 별도 명령으로 제공하면
SC-006의 2분 내 식별 가능성과 부분 준비 환경 edge case를 직접 다룰 수 있다.

**Alternatives considered**:

- README에 수동 체크 목록만 제공: 오류를 늦게 발견하며 환경별 메시지가 일관되지 않다.
- install script가 SDK까지 자동 설치: 큰 다운로드와 시스템 변경을 수행하므로 이 기능의
  안전한 초기화 범위를 넘는다.

**Sources**: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## 6. React 구조와 상태 소유권

**Decision**: `app`, `pages`, `widgets`, `features`, `entities`, `shared`를 저장소 루트의
`src`에 둔다. `QueryClientProvider`는 `app/providers`에서 한 번 조립한다. 이 기능에는
원격 데이터나 의미 있는 client state가 없으므로 query와 Zustand store를 만들지 않는다.

**Rationale**: provider 존재는 후속 기능의 일관된 composition point를 보장한다. 동시에
빈 store나 dummy query를 만들지 않으면 상태의 단일 진실 공급원 원칙을 지키고 초기 화면을
정적으로 유지할 수 있다.

**Alternatives considered**:

- 기술별 `components/hooks/stores` 구조: feature 경계와 import 방향을 드러내지 못한다.
- 초기 `useAppStore`: 소유할 실제 상태가 없으며 이후 domain state와 중복될 위험이 있다.
- Query provider도 나중으로 연기: 첫 remote feature가 임의 위치에서 provider를 만들 수 있다.

## 7. Rust hexagonal skeleton

**Decision**: `domain`, `application`, `ports`, `inbound`, `infrastructure` 모듈을 만들고
`lib.rs`만 composition root 역할을 한다. 이 기능에는 유스케이스가 없으므로 경계 설명과
architecture test만 만들고 fake command, entity, repository port는 만들지 않는다.

**Rationale**: 비어 있는 경계를 명시하면 후속 녹음 기능이 Tauri command에 규칙을 직접
넣는 것을 예방한다. 불필요한 trait와 DTO를 미리 정의하지 않으면 실제 요구에 맞춰 ports를
설계할 수 있다.

**Alternatives considered**:

- Handy의 `commands/managers/helpers` 구조 복제: 음성 앱 기능 아이디어는 참고할 수 있지만
  사용자 constitution의 hexagonal 경계를 만족하지 않는다.
- 각 계층을 독립 crate로 분리: 초기 shell 규모에서는 compile/workspace 복잡성만 증가한다.

## 8. UI와 검증 전략

**Decision**: shadcn `Card`를 사용한 단일 mobile-first 화면을 만들고 CSS safe-area inset,
`min-height: 100svh`, responsive padding을 적용한다. 자동 검증은 format, lint, typecheck,
frontend build/test, Rust fmt/clippy/test 및 doctor test를 포함한다. 실제 기기 checklist는
양 플랫폼 5회 cold launch, portrait/landscape, background/foreground, 권한 prompt 부재를
기록한다.

**Rationale**: 초기 shell의 실제 계약은 콘텐츠, 레이아웃, 무권한 실행과 개발 기반이다.
웹 단위 테스트만으로 native lifecycle, WebView, signing, device 연결을 증명할 수 없으므로
물리 기기 검증을 별도 완료 gate로 유지한다.

**Alternatives considered**:

- Playwright E2E를 초기 단계에 추가: native physical-device launch를 대체하지 못하고 현재
  화면 한 개에는 비용이 크다.
- 시각 snapshot만 사용: safe-area, 회전, 권한 prompt를 검증하지 못한다.

## 9. 보안과 권한

**Decision**: core shell 실행에 필요한 최소 Tauri capability만 두고 microphone, filesystem,
dialog, opener, HTTP 등 plugin과 mobile permission을 추가하지 않는다. `.env.example`도 API
키를 요구하지 않으며 로그에는 tool name/status만 허용한다.

**Rationale**: 이 feature는 transcription과 recording이 범위 밖이다. 사용하지 않는 권한과
plugin을 선제 추가하면 least privilege와 첫 실행 무권한 계약을 훼손한다.

**Alternatives considered**:

- 후속 녹음을 위해 microphone permission 선등록: 첫 실행에서 민감 권한을 요청하지 않는
  FR-011과 기능 범위 격리를 약화한다.
- OpenAI key placeholder 추가: client credential 배포를 정상 패턴처럼 보이게 한다.
