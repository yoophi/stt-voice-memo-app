# Implementation Plan: Minimal Android Tauri Host

**Branch**: `024-android-tauri-host` | **Date**: 2026-08-16 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-android-tauri-host/spec.md`

## Summary

Initialize and track the Tauri 2 Android Studio host at
`src-tauri/gen/android`, keep API 24 in `tauri.conf.json` as the platform-floor
authority, and reduce the generated host to a foreground touch launcher with no
permissions, providers, services, receivers, television launcher, or native
recording behavior. Repository validation will treat the Android host as a
fail-closed capability allowlist, distinguish absent/partial/invalid/verified
states, and expose stable root build and validation commands. Automated APK
build evidence is completed here; physical installation and launch remain owned
by Issue #23.

## Technical Context

**Language/Version**: Kotlin 1.9.25 generated host, Gradle Kotlin DSL, Rust stable edition 2024, TypeScript 5.9 on Node 22
**Primary Dependencies**: Tauri CLI 2.11.4, Tauri 2.11.3, Android Gradle Plugin 8.11.0, Gradle wrapper, Android SDK 36/NDK 28.2.13676358, pnpm 11
**Storage**: Tracked Android project source only; local SDK, Gradle caches, build artifacts, and signing files remain ignored
**Testing**: Vitest repository contract tests, capability/path validator fixtures, Tauri Android APK build, existing frontend/Rust/Swift/workspace validation
**Target Platform**: Android API 24+ foreground app shell; iOS 15+ must remain unchanged
**Project Type**: Mobile Tauri application in a pnpm/Cargo monorepo
**Performance Goals**: No runtime performance change; three consecutive clean host builds without initialization prompts
**Constraints**: No sensitive permissions, no network permission, no FileProvider, no Leanback/TV launcher, no recorder/service work, no secrets, no new React or Rust domain state
**Scale/Scope**: One generated Android host, one activity, one launcher intent filter, repository validation and documentation; physical validation deferred to Issue #23

## Constitution Check

_GATE: Passed before Phase 0 and re-checked after Phase 1 design._

- **Mobile first — PASS WITH TRACKED HANDOFF**: The design targets Android API
  24+ foreground launch and explicitly forbids microphone/background behavior.
  Automated build and capability evidence is completed in this feature. Physical
  Android install/launch evidence is a temporary completion exception owned by
  Issue #23 and cannot be represented as complete here.
- **Hexagonal Rust — PASS**: No domain, application, or port change is planned.
  Generated Kotlin/Gradle files are platform infrastructure and composition.
- **Feature-Sliced React — PASS**: No React slice or client state changes.
- **Secure transcription — PASS**: The manifest permission allowlist is empty,
  component exposure is exact, and existing built-client secret checks cover the
  host. No audio or transcript is created.
- **Resilient voice flow — NOT APPLICABLE / PASS**: This feature initializes only
  a foreground shell. The validator fails closed for absent, partial, and invalid
  hosts; Issue #23 owns physical success and failure evidence.

## Project Structure

### Documentation (this feature)

```text
specs/006-android-tauri-host/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── android-host-baseline.md
│   ├── capability-allowlist.md
│   └── root-android-workflow.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/
├── tauri.conf.json
└── gen/android/
    ├── app/
    │   ├── build.gradle.kts
    │   └── src/main/
    │       ├── AndroidManifest.xml
    │       ├── java/com/yoophi/sttvoicememo/MainActivity.kt
    │       └── res/
    ├── buildSrc/
    ├── gradle/wrapper/
    ├── build.gradle.kts
    ├── gradle.properties
    ├── gradlew
    ├── gradlew.bat
    └── settings.gradle

scripts/workspace/
├── check-mobile-paths.mjs
├── workspace-contract.test.mjs
└── fixtures/mobile-paths/

docs/
└── android-tauri-host.md

tests/device/
└── android-tauri-host.md
```

**Structure Decision**: Keep the project-pinned Tauri-generated Android host in
its standard stable path and edit only platform infrastructure. The repository
validator owns capability policy so application code cannot silently broaden the
manifest. No parallel Android project, React state, or Rust domain abstraction is
introduced.

## Design Decisions

### Generation and tracking

- Generate once with `pnpm tauri android init --ci` from the repository root.
- Commit host source, wrapper, build logic, activity, manifest, and launcher
  resources. Ignore local/build/signing outputs only.
- Regeneration uses the same lockfile-pinned CLI and configuration; it is a
  reviewed maintenance action, not a build prerequisite or byte-for-byte promise
  across CLI releases.

### Capability boundary

- The application manifest declares no `<uses-permission>` and exactly one
  required touchscreen `<uses-feature>` to make phone/tablet scope explicit.
- Exactly one exported `MainActivity` owns the `MAIN` + `LAUNCHER` filter.
- App-owned providers, services, receivers, aliases, Leanback categories, and
  background components are forbidden. The merged APK has a separate exact
  allowlist for non-exported or signature-guarded AndroidX runtime components.
- `MainActivity` extends `TauriActivity` without extra lifecycle policy such as
  edge-to-edge setup or recorder initialization.
- Validation inspects both authored host files and the packaged APK manifest,
  because Android library manifests are merged during packaging.

### Build and evidence

- Root commands remain the only contributor interface. A bundled debug ARM64 APK is the
  automated build gate; release signing and Play AAB publication are not required.
- Development-server execution would require a separately reviewed debug network
  permission overlay and is not an acceptance path for this permission-free host.
- Evidence records revision, exact command, toolchain versions, artifact path and
  SHA-256, and automated/physical classification without secrets or user content.
- Physical install, cold launch, API 24 device behavior, permission inspection,
  and unsupported-plugin behavior are recorded only in Issue #23.

## Constitution Check After Design

The Phase 1 design preserves all architecture and security boundaries. The only
open constitution gate is physical Android evidence. It is deliberately visible
as an unchecked handoff rather than a successful result; no task in this feature
may mark Issue #23 evidence complete.

## Complexity Tracking

| Violation                                                                     | Why Needed                                                                                                                                   | Simpler Alternative Rejected Because                                                                                                             | Termination Condition                                                                                                                   |
| ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Physical-device completion is deferred from this platform-host implementation | The user explicitly assigned real-device verification to GitHub Issue #23; #24 must make that issue executable without claiming its evidence | Claiming simulator/build output as device completion would violate Constitution I/V; blocking all host work would keep #23 impossible to execute | Issue #23 passes build, install, foreground/cold launch, permission, and sanitized-failure trials on an API 24+ physical Android device |
