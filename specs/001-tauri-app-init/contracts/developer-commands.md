# Contract: Developer Commands

## Package and reproducibility

| Command | Contract |
|---|---|
| `pnpm install --frozen-lockfile` | lockfile 그대로 설치하며 변경이 필요하면 실패한다. |
| `pnpm install` | 중단 후 안전하게 재실행 가능하며 생성 mobile source를 수동 삭제하지 않는다. |

`package.json`은 pnpm 버전을 고정하며 `pnpm-lock.yaml`과 `src-tauri/Cargo.lock`을 소스
관리한다. dependency cache와 build output은 소스 관리하지 않는다.

## Development and validation

| Script | Required behavior |
|---|---|
| `pnpm dev` | Vite shell을 strict port 1420에서 실행; 점유 시 명확히 실패한다. |
| `pnpm build` | TypeScript project build 후 production frontend bundle을 생성한다. |
| `pnpm format:check` | Prettier와 `cargo fmt --check`를 실행한다. |
| `pnpm lint` | ESLint와 Rust clippy를 warning-as-error로 실행한다. |
| `pnpm typecheck` | emit 없이 TypeScript를 검사한다. |
| `pnpm test` | Vitest와 `cargo test`를 실행한다. |
| `pnpm check` | format, lint, typecheck, test, build를 fail-fast로 실행한다. |
| `pnpm tauri info` | Tauri가 인식한 host/tool/project 정보를 표시한다. |

모든 검증 command는 exit code 0을 성공, non-zero를 실패로 사용한다. 실패 메시지는 최소한
실패 단계와 재실행할 수 있는 구체적 command를 포함한다.

## Mobile doctor

| Script | Scope |
|---|---|
| `pnpm doctor` | common + iOS + Android를 모두 검사하고 각 플랫폼 결과를 독립 표시한다. |
| `pnpm doctor:ios` | Node/pnpm/Rust, Xcode, CocoaPods, iOS Rust targets, 연결 기기를 검사한다. |
| `pnpm doctor:android` | Node/pnpm/Rust, Java, Android SDK/NDK, adb, Android Rust targets, 연결 기기를 검사한다. |

### Output contract

- 각 check는 `[PASS]`, `[WARN]`, `[FAIL]`, `[SKIP]` 중 하나와 stable check id를 출력한다.
- `FAIL`은 누락한 prerequisite와 공식 설치 문서 또는 다음 실행 command를 포함한다.
- secret/environment value 자체는 출력하지 않고 설정 여부나 안전한 경로 존재 여부만 알린다.
- 선택한 target의 필수 check가 하나라도 실패하면 non-zero로 종료한다.
- `doctor` 전체 실행에서 iOS 실패와 Android 성공 또는 그 반대가 각각 구분되어야 한다.

## Mobile project lifecycle

| Script | Underlying command | Contract |
|---|---|---|
| `pnpm mobile:ios:init` | `tauri ios init` | `src-tauri/gen/apple`을 공식 template로 생성/복구한다. |
| `pnpm mobile:android:init` | `tauri android init` | `src-tauri/gen/android`를 공식 template로 생성/복구한다. |
| `pnpm mobile:ios:dev` | `tauri ios dev` | 선택한 실제 iOS 기기에 development app을 실행한다. |
| `pnpm mobile:android:dev` | `tauri android dev` | 선택한 실제 Android 기기에 development app을 실행한다. |

실제 iOS 기기 연결 문제가 있으면 Xcode device 연결 후
`pnpm tauri ios dev --force-ip-prompt`를 사용할 수 있다. IDE debugging에는
`pnpm tauri [ios|android] dev --open`을 사용하며 CLI process를 종료하지 않는다.

## Explicit exclusions

이 feature의 command에는 API key 설정, backend 시작, microphone permission 설정, audio
capture, transcription 또는 desktop release build가 없다.
