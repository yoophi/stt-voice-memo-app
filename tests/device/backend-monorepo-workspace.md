# Backend Monorepo Workspace — Physical Device Evidence

**Feature**: `specs/005-backend-monorepo-workspace/spec.md`

**Rule**: Automated, simulator, and unsigned-build checks do not complete the
physical-device gate. Record only synthetic/content-free identifiers. Never add
credentials, audio, transcript text, private paths, or signing material.

## Tested revision

- Validated implementation commit: `5e4fb92`
- Date: 2026-08-15
- Validator: Codex automated validation on macOS; no physical iPhone or Android
  device was connected, so physical-device rows remain unverified

## Physical iPhone

| Field                                                | Evidence |
| ---------------------------------------------------- | -------- |
| Device model                                         | Not run  |
| iOS version                                          | Not run  |
| Root command                                         | Not run  |
| Build                                                | Not run  |
| Install                                              | Not run  |
| Foreground launch                                    | Not run  |
| Existing recorder availability                       | Not run  |
| No new permission prompt                             | Not run  |
| No backend-only configuration in inspected app/build | Not run  |

## Physical Android

| Field                                                | Evidence |
| ---------------------------------------------------- | -------- |
| Device model                                         | Not run  |
| Android/API version                                  | Not run  |
| Root command                                         | Not run  |
| Build                                                | Not run  |
| Install                                              | Not run  |
| Foreground launch                                    | Not run  |
| Existing unsupported-recorder result remains bounded | Not run  |
| No new permission prompt                             | Not run  |
| No backend-only configuration in inspected app/build | Not run  |

## Automated migration evidence

| Command                                                                  | Commit    | Outcome | Notes                                                                                |
| ------------------------------------------------------------------------ | --------- | ------- | ------------------------------------------------------------------------------------ |
| `pnpm test:workspace`                                                    | `5e4fb92` | Passed  | 24 workspace/path/security tests                                                     |
| `pnpm validate:contract`                                                 | `5e4fb92` | Passed  | 49 contract/workspace tests; generated contract current                              |
| `pnpm validate:backend`                                                  | `5e4fb92` | Passed  | Scaffold and boundaries only; no runtime claim                                       |
| `pnpm validate:mobile`                                                   | `5e4fb92` | Passed  | 18 frontend tests, boundary check, 64 Rust tests, both mobile paths, and secret scan |
| `pnpm lint:rust`                                                         | `5e4fb92` | Passed  | Workspace/all-target Clippy; vendored `swift-rs` warnings only                       |
| Clean checkout: `pnpm install --frozen-lockfile && pnpm test:swift`      | `5e4fb92` | Passed  | Generated ignored Tauri Swift API, then passed 17 simulator tests                    |
| `pnpm tauri ios build --debug --target aarch64 --no-sign`                | `5e4fb92` | Passed  | Unsigned IPA remained under `src-tauri/gen/apple/build/arm64`                        |
| `JAVA_HOME=$(/usr/libexec/java_home -v 17) pnpm tauri android build ...` | `5e4fb92` | Passed  | arm64 debug APK remained under `src-tauri/gen/android/app/build/outputs/apk`         |
| `pnpm validate`                                                          | `5e4fb92` | Passed  | Full automated repository validation                                                 |
| Content/secret/path review                                               | `5e4fb92` | Passed  | Plain and transformed synthetic canaries only; no real secret or mobile source move  |

## Completion

- [ ] T037 physical iPhone build/install/launch evidence is complete.
- [ ] T038 physical Android build/install/launch evidence is complete.
- [ ] No newly requested permission or backend-only configuration was observed.
