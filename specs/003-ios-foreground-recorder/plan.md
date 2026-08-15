# Implementation Plan: iOS Foreground Recorder Adapter

**Branch**: `004` | **Date**: 2026-08-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from
`/specs/003-ios-foreground-recorder/spec.md`

## Summary

Implement a local Tauri 2 `recorder` mobile plugin whose stable Rust/TypeScript
surface delegates iOS work to an injected Swift recording coordinator. Keep
recorder state and transition rules in a pure Rust `recorder-core` crate, use
`AVAudioRecorder` for foreground AAC-in-M4A capture, normalize permission and
lifecycle outcomes, expose native terminal events, and grant only the plugin
commands required by this issue. Runtime upload, memo UI, Android, background
audio, and relaunch orchestration remain deferred.

## Technical Context

**Language/Version**: Rust stable edition 2024 (minimum 1.85), Swift 5.9+
compatible with the generated iOS 15+ host, TypeScript 5.9 for the plugin guest
API

**Primary Dependencies**: Tauri 2.11 plugin API, `tauri-plugin` 2.6 build helper,
Serde 1, thiserror 2, Apple AVFAudio/Foundation/UIKit; no third-party recording
or codec dependency

**Storage**: App-private iOS `Application Support/Recordings` directory with
temporary `.m4a` files promoted in place to finalized source recordings; no
database and no upload

**Testing**: `cargo test` for pure contract/state tests, Swift Testing/XCTest for
the injected iOS coordinator where supported, TypeScript build/lint checks, and
the physical-iPhone matrix in `quickstart.md`

**Target Platform**: iPhone on iOS 15+; host macOS and Android builds receive an
explicit unsupported-platform adapter, with Android startup regression checked
because plugin initialization is shared

**Project Type**: Mobile-first Tauri application with an in-repository native
mobile plugin and a pure Rust core crate

**Performance Goals**: Permission/invalid-transition outcomes return within one
second after OS/user completion; stop finalizes a typical short memo without a
hung state; twenty sequential recording trials produce one result each

**Constraints**: Foreground-only; one active session; no raw path/audio/native
error logging; `.m4a`/`audio/mp4`; no background entitlement; terminal actions
idempotent; Swift callbacks and notification races serialized on the main actor

**Scale/Scope**: One recorder, one iOS adapter, eight commands, one native event
stream, short voice memos, and physical-device validation on at least one iPhone

## Constitution Check

_GATE: Passed before Phase 0 and re-checked after Phase 1 design._

- **Mobile first — PASS**: iOS 15+ permission, audio session, route,
  interruption, foreground exit, and physical-device evidence are explicit.
  Native Android recording remains excluded, while shared plugin initialization
  is required to cold-start safely and expose only `unsupportedPlatform` there;
  background, realtime, and desktop recording are excluded.
- **Hexagonal Rust — PASS**: `recorder-core` owns pure models, transition rules,
  the `RecorderPort`, and use-case coordination without Tauri, filesystem, or OS
  dependencies. The Tauri plugin and Swift package are inbound/infrastructure
  adapters. Swift is authoritative for live platform state; Rust mirrors the
  portable lifecycle and refreshes from native status before reporting status or
  replacing a possibly stale active session.
- **Feature-Sliced React — PASS**: No UI or state store is introduced. The
  optional guest API lives in `src/shared/api/recorder` as a platform client;
  Issue #6 will own recording-session Zustand state and event reconciliation.
- **Secure transcription — PASS WITH TEMPORARY EXCEPTION**: No provider, backend,
  credential, network, or transcript code is added. Audio stays app-private;
  logs carry only session IDs and sanitized codes; cancel/failure cleanup is
  explicit. Issue #4 cannot provide a safe user retention choice before a
  consumer owns the finalized artifact, so the bounded exception and its Issue
  #6 termination conditions are recorded below and in `spec.md`.
- **Resilient voice flow — PASS**: The state machine, stable session IDs,
  idempotent stop/cancel, interruption/route/background terminal outcomes,
  contract tests, and physical-iPhone matrix are defined.

### Post-design re-check

The Phase 1 data model and contracts preserve every gate. In particular, the
native `fileUri` is returned only across the trusted Rust plugin boundary and is
not logged or exposed as an analytics field; React receives an opaque
`artifactId` plus sanitized metadata. The capability grants individual recorder
commands and adds no general filesystem permission.

## Project Structure

### Documentation (this feature)

```text
specs/003-ios-foreground-recorder/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── recorder-plugin.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
└── shared/
    └── api/
        └── recorder/
            ├── index.ts
            ├── recorder-client.ts
            └── recorder-client.test.ts

src-tauri/
├── Cargo.toml
├── capabilities/
│   └── default.json
├── crates/
│   └── recorder-core/
│       ├── Cargo.toml
│       └── src/
│           ├── application.rs
│           ├── domain.rs
│           ├── lib.rs
│           └── ports.rs
├── plugins/
│   └── recorder/
│       ├── Cargo.toml
│       ├── build.rs
│       ├── permissions/default.toml
│       ├── src/
│       │   ├── commands.rs
│       │   ├── desktop.rs
│       │   ├── error.rs
│       │   ├── lib.rs
│       │   ├── mobile.rs
│       │   └── models.rs
│       └── ios/
│           ├── Package.swift
│           ├── Sources/
│           │   ├── RecorderEngine.swift
│           │   ├── RecorderPlugin.swift
│           │   └── RecorderTypes.swift
│           └── Tests/PluginTests/
│               └── RecorderCoordinatorTests.swift
├── gen/apple/stt-voice-memo-app_iOS/Info.plist
└── src/lib.rs

tests/
└── device/
    └── ios-foreground-recorder.md
```

**Structure Decision**: A pure `recorder-core` crate creates an enforceable
compile boundary for domain/application/port code. The sibling local Tauri
plugin owns IPC, permissions, desktop rejection, and the Swift package. The
application registers the plugin in `src-tauri/src/lib.rs`; React imports only
the public `src/shared/api/recorder` API. Generated Xcode project files receive
only the required microphone usage description, never a background-audio mode.

## Implementation Phases

### Phase 0 — Research

- Confirm Tauri 2 mobile plugin registration, generated command permissions,
  Swift listener events, and in-repository path dependency layout.
- Confirm iOS 15-compatible microphone permission APIs, audio-session category,
  `AVAudioRecorder` AAC/M4A behavior, and lifecycle notification semantics.
- Record decisions and rejected alternatives in `research.md`.

### Phase 1 — Design and contracts

- Define normalized entities, state transitions, descriptor validation, cleanup
  outcomes, and event sequencing in `data-model.md`.
- Define command inputs/results/errors/events and least-privilege permissions in
  `contracts/recorder-plugin.md`.
- Define automated and physical-device validation in `quickstart.md`.
- Re-run all constitution gates after completing the design.

### Phase 2 — Task planning and implementation

- Build the pure Rust core contract test-first.
- Scaffold and implement the Tauri plugin and least-privilege capability.
- Implement the Swift coordinator behind injected file/audio/recorder services,
  then connect `AVAudioSession`, `AVAudioRecorder`, and native notifications.
- Add the typed shared client and validation evidence template.
- Run Rust, TypeScript, Swift/iOS build, formatting, and contract checks; physical
  scenarios remain explicitly unverified if no iPhone can be operated from this
  environment.

## Complexity Tracking

| Exception                                                                               | Why required now                                                                                                                            | Rejected alternative                                                                                                                                  | Termination condition                                                                                                                                                                                                                                                                    |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A finalized source recording has no user-visible retention/deletion control in Issue #4 | The recorder adapter must hand one verified local artifact to the later upload journey, but owns no UI, upload, memo, or durable preference | Automatically deleting on stop would destroy the only source before Issue #6 can consume it; adding retention UI here would violate feature ownership | Issue #6 presents the retention choice, deletes unretained audio after handoff, exposes cleanup recovery, and records physical-device evidence. Issue #7 keeps source-audio deletion independent from memo deletion. The complete journey cannot be production-ready before this closes. |

The additional pure Rust crate is the smallest enforceable boundary that keeps
domain/application tests independent of Tauri and iOS, as required by Principle
II. Issue #4 owns this minimal recorder port and lifecycle contract because the
iOS adapter cannot be tested or integrated without it; Issue #5 owns the
separate recording-file access and backend transcription ports.
