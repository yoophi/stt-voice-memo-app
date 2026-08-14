# Quickstart Validation: Mobile Tauri App Foundation

이 문서는 구현 완료 후 새 checkout에서 foundation을 검증하는 실행 가이드다. 명령별 자세한
출력 계약은 [developer-commands.md](./contracts/developer-commands.md), 화면 계약은
[app-shell.md](./contracts/app-shell.md)를 따른다.

## 1. Prerequisites

공통:

- macOS 개발 호스트
- Node.js 22.22 이상, Corepack, pnpm 11.0.9
- stable Rust toolchain
- 실제 iOS 기기와 실제 Android 기기

iOS:

- 전체 Xcode 설치 및 최초 실행 완료
- CocoaPods
- `aarch64-apple-ios`, `x86_64-apple-ios`, `aarch64-apple-ios-sim` Rust targets
- Apple development signing이 가능한 연결 기기

Android:

- Android Studio와 bundled JBR/JDK
- Android SDK Platform/Platform-Tools/Build-Tools/Command-line Tools 및 NDK
- `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME`
- Android Rust targets와 USB debugging이 활성화된 연결 기기

공식 설치 절차는 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)를 따른다.

## 2. Clean checkout setup

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm tauri info
```

Expected:

- lockfile 변경 없이 설치된다.
- Tauri가 Node, pnpm, Rust 및 프로젝트 config를 인식한다.
- API key나 `.env` secret을 요구하지 않는다.

중단된 설치는 `pnpm install`을 다시 실행해 복구한다. `node_modules`, Cargo cache 또는
`src-tauri/gen`을 먼저 삭제하는 절차를 기본 복구법으로 사용하지 않는다.

## 3. Automated foundation validation

```bash
pnpm check
```

Expected: format, lint, TypeScript, Vitest, Rust test 및 frontend production build가 모두
exit code 0으로 끝난다.

별도 진단이 필요하면 다음 명령을 실행한다.

```bash
pnpm doctor:ios
pnpm doctor:android
```

Expected: 준비된 플랫폼은 모든 필수 check가 `PASS`이고 연결 기기가 표시된다. 준비되지 않은
플랫폼은 stable check id, 누락 prerequisite, 다음 조치를 `FAIL`로 표시하며 다른 플랫폼
결과를 가리지 않는다.

현재 계획 시점의 이 머신에서는 Android `java`가 탐지되지 않았으므로 구현 검증 전에
Android Studio JBR을 설치하고 `JAVA_HOME`을 설정해야 한다.

## 4. iOS physical-device validation

최초 한 번 또는 생성 project가 누락된 경우:

```bash
pnpm mobile:ios:init
```

실제 기기를 연결하고 Xcode signing team을 확인한 다음:

```bash
pnpm mobile:ios:dev
```

기기가 development server에 연결되지 않으면 Xcode의 Devices and Simulators에서 network
연결을 확인하고 다음 공식 fallback을 사용한다.

```bash
pnpm tauri ios dev --force-ip-prompt
```

Expected:

- 실제 iOS 기기에 `STT Voice Memo`가 설치되고 초기 shell이 표시된다.
- 민감한 permission prompt가 없다.
- portrait/landscape와 background/foreground 복귀가 화면을 손상시키지 않는다.
- 앱을 완전히 종료한 뒤 5회 연속 cold launch가 성공한다.

결과를 `tests/device/mobile-shell-smoke.md`에 기록한다.

## 5. Android physical-device validation

최초 한 번 또는 생성 project가 누락된 경우:

```bash
pnpm mobile:android:init
```

USB debugging 기기를 연결하고 `adb devices`에서 승인 상태를 확인한 다음:

```bash
pnpm mobile:android:dev
```

Expected:

- 실제 Android 기기에 `STT Voice Memo`가 설치되고 초기 shell이 표시된다.
- 민감한 permission prompt가 없다.
- portrait/landscape와 background/foreground 복귀가 화면을 손상시키지 않는다.
- 앱을 완전히 종료한 뒤 5회 연속 cold launch가 성공한다.

결과를 `tests/device/mobile-shell-smoke.md`에 기록한다.

## 6. Edge-case validation

### Port conflict

다른 process가 port 1420을 점유한 상태에서 `pnpm dev`를 실행한다.

Expected: Vite가 임의의 다른 port로 이동하지 않고 port 1420 충돌을 명확히 출력한다. 기존
process를 종료한 뒤 같은 명령을 재실행하면 성공한다.

### One platform unavailable

한 플랫폼 도구를 사용할 수 없는 환경에서 준비된 다른 플랫폼의 doctor와 dev command를
실행한다.

Expected: 준비된 플랫폼은 계속 실행되며, 누락 플랫폼만 prerequisite failure로 분류된다.

### Minimum viewport

frontend test viewport를 `320 × 568`로 설정하고 portrait/landscape를 확인한다.

Expected: heading과 ready 설명이 safe area 안에 있고 clipping/overlap이 없다.

## Completion evidence

다음이 모두 충족되어야 이 feature를 완료로 판단한다.

- `pnpm install --frozen-lockfile` 성공
- `pnpm check` 성공
- iOS/Android doctor 성공
- 실제 iOS 기기 smoke record 통과
- 실제 Android 기기 smoke record 통과
- 각 플랫폼 5회 cold launch 및 예상치 못한 permission prompt 0회
