# Validation Quickstart: Transcription Upload Use Case

## Prerequisites

- Node 22.22+, pnpm 11, Rust stable 1.85+, and the repository-supported Tauri
  mobile toolchains
- No OpenAI key, provider credential, production backend token, or live recording
  is required
- A deterministic non-sensitive m4a fixture for adapter/device validation
- For physical validation: signed iPhone with iOS 15+, Android API 24+ device,
  and a non-production HTTPS backend implementing the Issue #3 contract

## Automated validation

Run from the repository root:

```sh
pnpm install --frozen-lockfile
pnpm exec prettier --check .
pnpm exec eslint .
pnpm exec tsc -b
pnpm exec vitest run
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --workspace
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
```

Focused suites after implementation:

```sh
cargo test --manifest-path src-tauri/Cargo.toml -p transcription-core
cargo test --manifest-path src-tauri/Cargo.toml --test transcription_http_contract
pnpm exec vitest run scripts/backend-transcription-api-contract.test.mjs scripts/record-transcribe-contract.test.mjs
```

Expected outcomes:

- Core tests cover every state/terminal transition, exact replay, duplicate
  submit, offline, retry timing, restart recovery, cancel/completion races, stale
  progress, malformed result, and persistence failure without network.
- HTTP contract tests verify multipart field names, sanitized filename/MIME,
  Bearer/idempotency headers, 200/202 operation parsing, Retry-After, all stable
  error categories, timeout, cancellation, and malformed response against an
  in-process loopback server.
- Canary tests prove that token, path, audio, transcript, signed URL, and raw
  backend/provider payloads never occur in logs, events, errors, or persisted
  operation records.

## Deterministic scenario matrix

| Scenario                         | Expected result                                                                    |
| -------------------------------- | ---------------------------------------------------------------------------------- |
| First valid submit               | Intent persisted before one streamed create; accepted operation stores backend ID  |
| Repeated submit                  | Existing local operation/result; no second logical provider operation              |
| Offline before submit            | `waitingForNetwork`; zero HTTP requests                                            |
| Lost create response             | `uncertain`; exact replay with same key/fingerprint resolves existing operation    |
| Known backend ID timeout         | GET status before any create replay                                                |
| Retryable 429/503                | Safe retry time honored; same identity and bounded attempt                         |
| 401/auth expiry                  | User-actionable failure; no automatic retry or token persistence                   |
| Cancel before upload             | Local cancelled winner; zero HTTP requests                                         |
| Cancel during upload             | Local token stops transfer; DELETE/reconciliation decides remote outcome           |
| Cancel versus completion         | First durable terminal winner is immutable in every ordering                       |
| Process death per phase          | Relaunch recovers same content-free operation and requires at most one user action |
| Malformed/blank completed result | Fail closed; no transcript result                                                  |
| Source changed/disappeared       | Fail before network; retain stable operation/failure                               |

## Content-safety inspection

Inject unique canaries for authorization token, transcript, absolute path, audio
bytes/base64, signed URL, and raw backend/provider error. Exercise every success,
failure, retry, cancellation, and recovery branch. Serialize all observable logs,
events, command errors, and operation record files and assert no canary or
forbidden field name appears.

Allowed diagnostics are limited to operation/source IDs, phase, attempt, byte
counts, stable error code/category, safe backend request ID, elapsed time, and
retry delay.

## Physical iOS and Android validation

Use a debug/test-only source adapter to copy the same 1–3 second non-sensitive
fixture into app-private storage. Inject an expiring non-production access token
at runtime; never commit it. Do not request microphone permission.

On each physical device:

1. Submit the fixture and resolve one completed non-blank transcript.
2. Repeat submit and verify one logical/backend operation.
3. Start offline and recover within one explicit retry after connectivity returns.
4. Lose the create response or interrupt the network mid-upload and verify exact
   identity recovery without duplicate provider work.
5. Relaunch after persisted intent and during polling; recover the same operation.
6. Cancel during upload/processing and verify one terminal winner plus observable
   cleanup.
7. Confirm the app performs no unsupported background transfer and requests no
   microphone permission.

Record device model, OS version, build commit, fixture metadata/checksum, backend
fixture version, scenario, content-free operation/request IDs, and observed result
in `tests/device/transcription-upload-usecase.md`. Never record the spoken phrase,
transcript, token, path, or audio content.

## Completion gate

Implementation can be code-complete after automated tests and mobile builds, but
Issue #5 is not acceptance-complete until the physical iPhone and Android success
plus offline/uncertain recovery evidence passes. Production backend/provider and
recorder-to-source integration remain separate work.

## Implementation evidence (2026-08-15)

- `cargo test --manifest-path src-tauri/Cargo.toml --workspace`: passed; the
  transcription core contributed 14 state/use-case/recovery tests and the HTTP
  loopback suite contributed 5 transport contract tests.
- Focused repository contracts: 35/35 passed, including 6 Issue #5 artifact and
  architecture checks.
- `cargo fmt --check`, strict workspace clippy, `tsc -b`, ESLint, and targeted
  Prettier checks passed. The patched upstream `swift-rs` dependency still emits
  its two pre-existing warnings outside project code.
- The full-repository Prettier check remains red because pre-existing Spec Kit,
  historical spec/doc, and user-owned `.wtp.yml` files are not formatted; all
  Issue #5 artifacts pass their targeted Prettier check. T034 remains open.
- `IPHONEOS_DEPLOYMENT_TARGET=15.0 cargo build --target
aarch64-apple-ios-sim` and the corresponding `aarch64-apple-ios` build passed.
  No simulator or physical device was launched.
- Android arm64 compilation was attempted but the local toolchain has no
  discoverable `aarch64-linux-android-clang`; install/configure the Android NDK
  before rerunning T035.
- T036 and T037 remain open because no physical device/backend fixture run was
  performed. This feature is implementation-complete for automated behavior but
  not acceptance-complete.
