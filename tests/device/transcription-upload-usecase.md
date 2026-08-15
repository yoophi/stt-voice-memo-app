# Transcription Upload Use Case Physical-Device Evidence

**Feature**: GitHub Issue #5 / `specs/004-transcription-upload-usecase/`

Record only observations from physical devices. Simulator, emulator, loopback,
and compile-only results are supplementary and do not satisfy the physical-device
completion gate.

Do not include the fixture phrase, transcript text, audio bytes, local or signed
paths, authorization values, credentials, provider payloads, or raw native/backend
errors in this document or linked evidence. Use only stable content-free operation
and backend request identifiers.

## Shared fixture and backend

| Field                                      | Result  |
| ------------------------------------------ | ------- |
| Fixture format                             | Not run |
| Fixture byte length                        | Not run |
| Fixture duration                           | Not run |
| Fixture SHA-256                            | Not run |
| Non-production backend fixture version     | Not run |
| Provider calls (must be 0)                 | Not run |
| Microphone permission requests (must be 0) | Not run |

The same deterministic, non-sensitive finalized-audio fixture and checksum must
be used on both platforms. Supply the non-production access token at runtime; do
not persist it or commit it as fixture data.

## Physical iPhone

### Environment

| Field                      | Result  |
| -------------------------- | ------- |
| Status                     | Not run |
| Device model               | Not run |
| iOS version (minimum 15.0) | Not run |
| Build commit               | Not run |
| Validation date (ISO-8601) | Not run |
| Tester                     | Not run |

### Acceptance matrix

| Scenario                                                                  | Status  | Content-safe evidence / notes |
| ------------------------------------------------------------------------- | ------- | ----------------------------- |
| Fixture submit reaches one completed non-blank result                     | Not run | Physical iPhone required      |
| Repeated submit preserves one logical/backend operation                   | Not run | Physical iPhone required      |
| Offline-before-submit performs zero HTTP requests and recovers explicitly | Not run | Physical iPhone required      |
| Lost create response or mid-upload timeout recovers the same identity     | Not run | Physical iPhone required      |
| Relaunch after durable intent recovers the same operation                 | Not run | Physical iPhone required      |
| Relaunch during queued/processing polling resolves by backend ID          | Not run | Physical iPhone required      |
| Cancel during upload/processing preserves one terminal winner and cleanup | Not run | Physical iPhone required      |
| Background/foreground performs no unsupported background transfer         | Not run | Physical iPhone required      |
| Device, app, and backend diagnostics contain no sensitive canary          | Not run | Physical iPhone required      |

## Physical Android

### Environment

| Field                                | Result  |
| ------------------------------------ | ------- |
| Status                               | Not run |
| Device model                         | Not run |
| Android/API version (minimum API 24) | Not run |
| System WebView version               | Not run |
| Build commit                         | Not run |
| Validation date (ISO-8601)           | Not run |
| Tester                               | Not run |

### Acceptance matrix

| Scenario                                                                  | Status  | Content-safe evidence / notes |
| ------------------------------------------------------------------------- | ------- | ----------------------------- |
| Fixture submit reaches one completed non-blank result                     | Not run | Physical Android required     |
| Repeated submit preserves one logical/backend operation                   | Not run | Physical Android required     |
| Offline-before-submit performs zero HTTP requests and recovers explicitly | Not run | Physical Android required     |
| Lost create response or mid-upload timeout recovers the same identity     | Not run | Physical Android required     |
| Relaunch after durable intent recovers the same operation                 | Not run | Physical Android required     |
| Relaunch during queued/processing polling resolves by backend ID          | Not run | Physical Android required     |
| Cancel during upload/processing preserves one terminal winner and cleanup | Not run | Physical Android required     |
| Background/foreground performs no unsupported background transfer         | Not run | Physical Android required     |
| Device, app, and backend diagnostics contain no sensitive canary          | Not run | Physical Android required     |

## Automated and build evidence

| Check                                            | Status  | Evidence / notes                                 |
| ------------------------------------------------ | ------- | ------------------------------------------------ |
| Issue #5 artifact/architecture contract          | Pass    | Vitest: 6/6                                      |
| Backend API and journey contract regressions     | Pass    | Full Vitest: 43/43                               |
| Transcription core state/use-case/recovery tests | Pass    | Core: 17/17; workspace suite passed              |
| HTTP multipart/status/delete contract tests      | Pass    | Loopback target: 11/11                           |
| Content-safety canary tests                      | Pass    | DTO, event, error, and persisted-record checks   |
| TypeScript build, lint, and formatting           | Pass    | `tsc`, ESLint, targeted Prettier                 |
| Rust formatting, workspace tests, and clippy     | Pass    | Strict project checks passed                     |
| iOS simulator compile/run                        | Partial | arm64 simulator compile passed; not launched     |
| Unsigned/signed iOS arm64 build                  | Pass    | Unsigned Rust/Tauri arm64 compile passed         |
| Android arm64 build                              | Blocked | Android NDK clang unavailable in local toolchain |

## Completion statement

Issue #5 must not be marked acceptance-complete until the success flow and at
least one offline or uncertain-timeout recovery flow pass on both a physical
iPhone and a physical Android API 24+ device. Every row above remains `Not run`
until actual content-safe evidence is recorded; automated, simulator, emulator,
or compile-only evidence cannot be substituted for a physical result.
