# Backend Monorepo Workspace — Physical Device Evidence

**Feature**: `specs/005-backend-monorepo-workspace/spec.md`

**Rule**: Automated, simulator, and unsigned-build checks do not complete the
physical-device gate. Record only synthetic/content-free identifiers. Never add
credentials, audio, transcript text, private paths, or signing material.

## Tested revision

- Commit: working tree based on `030950f`
- Date: 2026-08-15
- Validator: Codex automated validation; physical-device rows remain unverified

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

| Command                                                   | Commit       | Outcome | Notes                                                                                   |
| --------------------------------------------------------- | ------------ | ------- | --------------------------------------------------------------------------------------- |
| `pnpm test:workspace`                                     | working tree | Passed  | 19 workspace/path/security tests                                                        |
| `pnpm validate:contract`                                  | working tree | Passed  | 44 contract/workspace tests; drift current                                              |
| `pnpm validate:backend`                                   | working tree | Passed  | Scaffold and boundaries only; no runtime claim                                          |
| `pnpm validate:mobile`                                    | working tree | Passed  | Frontend build/tests/lint, 64 Rust tests, mobile paths and secret scan                  |
| `pnpm lint:rust`                                          | working tree | Passed  | Workspace/all-target Clippy; vendored `swift-rs` warnings only                          |
| `pnpm test:swift`                                         | working tree | Passed  | 17 coordinator tests on iOS Simulator; not physical evidence                            |
| `pnpm tauri ios build --debug --target aarch64 --no-sign` | working tree | Passed  | Unsigned bundle remained under `src-tauri/gen/apple`                                    |
| `pnpm tauri android build --debug`                        | working tree | Not run | Existing Android project is uninitialized at `src-tauri/gen/android`; `adb` unavailable |
| `pnpm validate`                                           | working tree | Passed  | Full automated repository validation                                                    |
| Content/secret/path review                                | working tree | Passed  | Synthetic canaries only; no real secret or mobile source move                           |

## Completion

- [ ] T037 physical iPhone build/install/launch evidence is complete.
- [ ] T038 physical Android build/install/launch evidence is complete.
- [ ] No newly requested permission or backend-only configuration was observed.
