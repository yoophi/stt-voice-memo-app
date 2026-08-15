# Backend Monorepo Workspace — Physical Device Evidence

**Feature**: `specs/005-backend-monorepo-workspace/spec.md`

**Rule**: Automated, simulator, and unsigned-build checks do not complete the
physical-device gate. Record only synthetic/content-free identifiers. Never add
credentials, audio, transcript text, private paths, or signing material.

**Scope**: Physical execution is excluded from PR #22 and owned by follow-up
GitHub Issue [#23](https://github.com/yoophi/stt-voice-memo-app/issues/23). The
`Not run` rows below are the starting template for that issue, not a completion
claim in this PR. Android execution depends on Issue
[#24](https://github.com/yoophi/stt-voice-memo-app/issues/24). Feature acceptance
remains incomplete while either platform is `Not run`.

## Tested revision

- Validated implementation commit: `5b57461`
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

| Command                                                             | Commit    | Outcome | Notes                                                                                |
| ------------------------------------------------------------------- | --------- | ------- | ------------------------------------------------------------------------------------ |
| `pnpm test:workspace`                                               | `5b57461` | Passed  | 25 workspace/path/security tests                                                     |
| `pnpm validate:contract`                                            | `5b57461` | Passed  | 50 contract/workspace tests; generated contract current                              |
| `pnpm validate:backend`                                             | `5b57461` | Passed  | Scaffold and boundaries only; no runtime claim                                       |
| `pnpm validate:mobile`                                              | `5b57461` | Passed  | 18 frontend, 64 Rust, 17 Swift, boundary, host-state, and actual-build canary checks |
| `pnpm lint:rust`                                                    | `5b57461` | Passed  | Workspace/all-target Clippy; vendored `swift-rs` warnings only                       |
| Clean checkout: `pnpm install --frozen-lockfile && pnpm test:swift` | `5e4fb92` | Passed  | Generated ignored Tauri Swift API, then passed 17 simulator tests                    |
| `pnpm tauri ios build --debug --target aarch64 --no-sign`           | `5e4fb92` | Passed  | Unsigned IPA remained under `src-tauri/gen/apple/build/arm64`                        |
| Android host availability                                           | `5b57461` | Partial | Checker reports `android=unavailable`; Issue #24 owns initialization                 |
| `pnpm validate`                                                     | `5b57461` | Passed  | Full automated repository validation                                                 |
| Actual-build secret boundary                                        | `5b57461` | Passed  | Unique canary transformed by Vite, detected without echo, temporary build removed    |

## Follow-up ownership

- [ ] T037 physical iPhone evidence passes in Issue #23.
- [ ] T038 physical Android evidence passes in Issue #23 after Issue #24.
- [ ] Issue #23 confirms no newly requested permission or backend-only
      configuration on physical devices.
