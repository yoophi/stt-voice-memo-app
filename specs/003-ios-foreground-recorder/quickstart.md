# Validation Quickstart: iOS Foreground Recorder Adapter

## Prerequisites

- macOS with the repository-supported Xcode and command-line tools
- Rust stable and the iOS Rust targets used by Tauri
- Node 22.22+ and pnpm 11
- A developer-signed physical iPhone running iOS 15 or later
- No audio content in screenshots, console captures, issue comments, or logs

## Automated contract validation

Run from the repository root:

```sh
pnpm install --frozen-lockfile
pnpm exec prettier --check src src-tauri/plugins/recorder/ios \
  specs/003-ios-foreground-recorder tests/device/ios-foreground-recorder.md
pnpm exec eslint .
pnpm exec tsc -b
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --workspace
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
```

The targeted Prettier scope avoids rewriting pre-existing generated, vendored,
Spec Kit, and historical feature files unrelated to Issue #4.

Expected outcome:

- Pure recorder-core tests cover all state transitions, idempotent terminal
  actions, permission outcomes, metadata validation, and cleanup classification.
- Plugin/guest API compilation exposes only the contract command names and
  sanitized public fields.
- Desktop tests return `unsupportedPlatform` without creating audio files.

## iOS build validation

```sh
pnpm tauri ios build --debug --target aarch64
```

Expected outcome:

- The local recorder Swift package is linked into the generated iOS app.
- The app contains `NSMicrophoneUsageDescription`.
- The app contains no background-audio mode.
- Capability generation recognizes the eight granted recorder commands and no
  general filesystem permission is added.

If the environment cannot sign/install the app, record the build result and
leave physical scenarios unchecked; a simulator run does not replace them.

## Physical-iPhone evidence record

Copy the matrix in `tests/device/ios-foreground-recorder.md` for the tested
commit. Record device model, iOS version, build commit, date, expected outcome,
actual outcome, and a content-free evidence reference.

| Scenario                                  |           Repetitions | Required result                                                             |
| ----------------------------------------- | --------------------: | --------------------------------------------------------------------------- |
| First Record permission grant             |       1 fresh install | One prompt after user action; capture begins only after grant               |
| Permission denial and repeat Record       |                     2 | No capture, no prompt loop, settings recovery indicated                     |
| Normal start/pause/resume/stop            |                    20 | One playable M4A each, positive duration/bytes, paused interval excluded    |
| User cancel                               |                     3 | No active session/audio session and no temporary artifact                   |
| Incoming call or system interruption      |                     2 | One terminal outcome, partial audio finalized when usable, no auto-resume   |
| Wired/Bluetooth input removal             | 2 per available route | One route-change terminal outcome; no unintended-mic continuation           |
| Home/app switch while recording           |                     3 | Finalize attempt and no background capture                                  |
| Media services reset (Developer settings) |                     1 | One reset failure/finalization outcome; next recording requires user action |
| Five consecutive cold launches            |                     5 | No crash/hung state; one successful recording after each launch             |
| Repeated stop/cancel taps                 |                3 each | One stored terminal outcome; no duplicate file/event                        |

## Artifact inspection

For each successful stop, verify through the trusted diagnostic harness (never
by logging paths or content):

- container/extension: `.m4a`;
- MIME type: `audio/mp4`;
- byte length and duration are positive;
- sample rate and channel count are present;
- checksum is a lowercase SHA-256 value;
- file is playable and contains the spoken test phrase;
- recorder and audio session are inactive after the result.

For cancel/failure cases, verify the session's temporary artifact is absent, or
that the returned cleanup outcome explicitly records pending/failed cleanup.

## Requirement traceability

| Acceptance area                | Primary requirements           | Evidence                                                                   |
| ------------------------------ | ------------------------------ | -------------------------------------------------------------------------- |
| Permission and least privilege | FR-001, FR-004, MLR-002        | Contract tests, Info.plist review, capability review, device denial/grant  |
| Normal capture/finalization    | FR-002–FR-008                  | Core tests, Swift tests, 20-run device trial, artifact inspection          |
| Cancel and idempotency         | FR-009–FR-010                  | Fake-port/Swift race tests, device cancel/repeated taps                    |
| Interruption/lifecycle         | FR-011–FR-013, MLR-003–MLR-005 | Swift notification tests and physical interruption/route/background matrix |
| Privacy/data lifecycle         | FR-014, PDL-001–PDL-005        | Source review, log-field assertions, cleanup inspection                    |
| Automated contract             | FR-015, SC-006                 | Workspace test and lint commands                                           |

## Completion gate

Implementation may be code-complete with automated and iOS build checks, but
GitHub Issue #4 is not fully acceptance-complete until the physical-iPhone
matrix includes permission denial, interruption, cancellation, five cold
launches, and successful recording evidence.

## Validation result (2026-08-15)

- Relevant Prettier files, ESLint, TypeScript build, and all 12 frontend tests
  passed.
- Rust formatting, all 16 workspace tests, and workspace clippy passed. The
  vendored `swift-rs` dependency emitted two non-fatal upstream warnings.
- All 11 Swift coordinator tests passed on the connected iOS simulator,
  including start-failure cleanup and explicit cleanup retry coverage.
- Rust compiled for both `aarch64-apple-ios-sim` and `aarch64-apple-ios` with
  `IPHONEOS_DEPLOYMENT_TARGET=15.0`.
- The recorder plugin compiled for `aarch64-linux-android`; Android startup uses
  the safe unsupported adapter rather than failing plugin initialization.
- `pnpm tauri ios build --debug --target aarch64 --no-sign` produced an unsigned
  iOS debug bundle successfully.
- `xcrun devicectl list devices` showed simulated devices only. Physical-device
  scenarios therefore remain deliberately unchecked in the evidence matrix.
