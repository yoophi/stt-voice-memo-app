# iOS Simulator 및 Xcode 27 트러블슈팅

## 문서 범위

2026-08-15에 Apple Silicon Mac에서 Tauri 앱을 iOS Simulator로 실행하면서 확인한
문제와 해결 과정을 기록한다. 이 내용은 다음 환경을 기준으로 한다.

- Tauri CLI 2.11.4
- Tauri Rust crate 2.11.5
- `swift-rs` 1.0.7
- Xcode 27.0 beta (`27A5228h`)
- iOS 26.4 Simulator
- Rust target `aarch64-apple-ios-sim`

## 최종 결과

`STT Voice Memo` 앱을 iOS 26.4 Simulator에 설치하고
`com.yoophi.sttvoicememo` 프로세스로 실행했다. React 앱 셸과 한국어 안내 문구가
정상적으로 렌더링되는 것도 Simulator screenshot으로 확인했다.

다만 Xcode 27과 현재 Tauri CLI 조합에서는 아래 두 가지 호환 문제가 남아 있어,
Swift 빌드용 로컬 patch와 `simctl` 직접 설치가 필요했다.

## 문제 1: Swift 빌드에 iOS와 macOS SDK가 동시에 전달됨

### 증상

다음 명령에서 Vite 개발 서버는 시작되지만 Tauri Swift package 컴파일이 실패했다.

```sh
pnpm tauri ios dev "iPhone 17 Pro"
```

주요 오류는 다음과 같았다.

```text
CoreServices/CSIdentityBase.h file not found
OpenGLES/EAGL.h file not found
UIKit/NSAttributedString.h file not found
unable to resolve module dependency 'UIKit'
unable to resolve module dependency 'AppKit'
unable to resolve module dependency 'WebKit'
```

실제 `swiftc` 호출에는 서로 충돌하는 SDK와 target이 함께 포함되어 있었다.

```text
-sdk .../iPhoneSimulator27.0.sdk
-target arm64-apple-ios15.0-simulator
...
-sdk .../MacOSX27.0.sdk
-target arm64-apple-macos12.0
```

### 원인

`swift-rs` 1.0.7은 SwiftPM에 `--arch arm64`를 전달한 뒤, 개별 Swift compiler에만
`-Xswiftc -sdk`와 `-Xswiftc -target`으로 iOS 설정을 덮어쓴다.

Xcode 27의 SwiftPM은 `--arch arm64`를 macOS host build로 계획하고 macOS SDK와
target을 compiler command 뒤쪽에 추가한다. 이 값이 앞에서 전달된 iOS Simulator
설정을 덮어쓰면서 UIKit과 AppKit이 섞인 module resolution 오류가 발생한다.

환경 변수 오염은 아니었다. 아래 변수는 설정되어 있지 않았다.

```text
SDKROOT
MACOSX_DEPLOYMENT_TARGET
IPHONEOS_DEPLOYMENT_TARGET
DEVELOPER_DIR
TOOLCHAINS
```

### 최소 재현과 수정 검증

Tauri의 Swift package만 분리해 SwiftPM에 target triple을 직접 전달하면 정상적으로
빌드되었다.

```sh
swift build \
  --triple arm64-apple-ios15.0-simulator \
  --sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)" \
  -c debug \
  --build-path /tmp/stt-voice-memo-tauri-swift-triple
```

### 프로젝트에 적용한 수정

crates.io의 `swift-rs` 1.0.7을 `src-tauri/vendor/swift-rs`에 vendoring하고
`src-tauri/Cargo.toml`에서 `[patch.crates-io]`로 해당 사본을 사용한다.

```toml
[patch.crates-io]
swift-rs = { path = "vendor/swift-rs" }
```

vendor patch는 다음 두 동작을 변경한다.

1. SwiftPM의 `--arch arm64` 대신 `--triple arm64-apple-ios15.0-simulator`를 사용한다.
2. Xcode 27이 생성하는 `out/Products/Debug-iphonesimulator/libTauri.a` 위치를 먼저
   확인하고, 이전 SwiftPM 산출 경로는 fallback으로 유지한다.

이 patch는 upstream `swift-rs`가 Xcode 27의 target-level `--triple` 방식을 지원하면
제거 여부를 재검토한다.

### 검증 명령

```sh
DEVELOPER_DIR=/Applications/Xcode-beta.app/Contents/Developer \
IPHONEOS_DEPLOYMENT_TARGET=15.0 \
cargo build \
  --manifest-path src-tauri/Cargo.toml \
  --target aarch64-apple-ios-sim \
  --lib \
  --no-default-features
```

수정 후 Tauri Swift static library와 iOS용 Rust library의 최종 링크가 통과했다.

## 문제 2: Simulator를 실제 iOS 기기 경로로 처리함

### 증상

Simulator용 `.app` build와 ad-hoc signing은 성공했지만, Tauri CLI가 이어서 archive를
시도하며 development team 오류로 종료됐다.

```text
** BUILD SUCCEEDED **
Archiving app...
Signing for "stt-voice-memo-app_iOS" requires a development team.
** ARCHIVE FAILED **
```

로그에서 Simulator가 `aarch64-apple-ios-sim`이 아니라 다음처럼 실제 기기 target으로
분류되었다.

```text
Detected connected device: ... with target "aarch64-apple-ios"
```

동일한 `iPhone 17 Pro` 이름을 가진 iOS 26.2와 26.4 Simulator가 있어, 우선 부팅된
기기에 고유 이름을 지정했다.

```sh
xcrun simctl rename <SIMULATOR_UDID> "STT Voice Memo iPhone"
```

이름 모호성은 제거됐지만 Xcode 27에서의 실제 기기 분류와 archive 시도는 계속됐다.
Simulator 실행에 development team을 추가하는 것은 필요 이상의 signing 설정이므로
사용하지 않았다.

### 현재 우회 절차

Tauri/Xcode build가 생성한 Simulator용 `.app`을 `simctl`로 직접 설치하고 실행한다.
`<APP_PATH>`는 Xcode build 로그의
`Build/Products/debug-iphonesimulator/STT Voice Memo.app` 경로를 사용한다.

```sh
xcrun simctl install <SIMULATOR_UDID> "<APP_PATH>"
xcrun simctl launch \
  <SIMULATOR_UDID> \
  com.yoophi.sttvoicememo
```

개발용 UI를 제공하려면 앱 실행 중 Vite server를 유지한다.

```sh
TAURI_DEV_HOST=<MAC_LAN_IP> pnpm dev
```

설치와 실행 상태는 다음 명령으로 확인할 수 있다.

```sh
xcrun simctl get_app_container \
  <SIMULATOR_UDID> \
  com.yoophi.sttvoicememo \
  app

xcrun simctl launch \
  <SIMULATOR_UDID> \
  com.yoophi.sttvoicememo
```

## Xcode 선택 과정에서 확인한 사항

시스템 `xcode-select`는 Xcode 27 beta를 가리키고 있었다.

```text
/Applications/Xcode-beta.app/Contents/Developer
```

명령 앞에 다음처럼 `DEVELOPER_DIR`를 지정해도 Tauri의 `cargo-mobile2`는
`xcode-select -p` 결과로 Xcode 실행 파일의 절대 경로를 다시 구성하므로 beta를
계속 사용했다.

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
pnpm tauri ios dev
```

안정판 Xcode 26.6의 `xcodebuild`를 직접 호출하는 방법도 확인했지만, 해당 Xcode에는
iOS 26.5 platform component가 완전히 설치되어 있지 않아 다음 오류로 중단됐다.

```text
iOS 26.5 is not installed.
Please download and install the platform from Xcode > Settings > Components.
```

전역 `xcode-select` 변경은 다른 Xcode 작업에 영향을 줄 수 있고 관리자 암호도
필요하므로 수행하지 않았다.

## 최종 검증 결과

```text
Frontend tests: 1 file, 2 tests passed
Frontend production build: passed
Desktop cargo check: passed
iOS aarch64-apple-ios-sim cargo build: passed
iOS Simulator install: passed
iOS Simulator process launch: passed
App shell screenshot inspection: passed
```

이 검증은 Simulator 실행 증거이며 실제 iOS 기기 검증을 대신하지 않는다. 마이크 권한,
오디오 session, interruption 및 lifecycle 동작은 추후 물리 기기에서 별도로 검증해야
한다.
