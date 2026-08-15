# Implementation Plan: Transcription Upload Use Case

**Branch**: `005-transcription-upload-usecase` | **Date**: 2026-08-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-transcription-upload-usecase/spec.md`

## Summary

Implement a compile-isolated Rust core that owns transcription operation
identity, state transitions, idempotency, retry, cancellation, and restart
recovery. The core uses outbound ports for trusted source audio, authorization,
durable operation records, event delivery, time, and the versioned backend. The
Tauri application supplies app-private atomic storage, streamed HTTPS multipart
transport, a content-safe event sink, and thin async commands. Recorder wiring,
production authentication, React Query integration, and memo persistence remain
in Issues #6 and #7.

## Technical Context

**Language/Version**: Rust stable 1.85+, edition 2024; TypeScript 5.9 only for existing contract regression tests

**Primary Dependencies**: `async-trait 0.1`, `serde 1`, `thiserror 2`, `uuid 1` in `transcription-core`; `reqwest 0.13` with `rustls`, `multipart`, `stream`, and `json`; `tokio 1`, `tokio-util 0.7`, `futures-util 0.3`, `serde_json 1`, and `sha2 0.10` in infrastructure; Tauri 2.11 inbound/event composition

**Storage**: App-private per-operation JSON records using temp-write, file sync,
atomic rename, parent-directory sync, revision compare-and-swap, and an
in-process keyed lock; transcript text, credentials, audio bytes, and file paths
are excluded from operation records

**Testing**: Rust unit tests with fake ports and clocks; in-process loopback HTTP
contract tests; existing Vitest API-contract tests; cargo fmt/clippy; physical
iPhone and Android fixture matrix

**Target Platform**: Mobile-first Tauri 2 on iOS 15+ and Android API 24+;
desktop remains an unsupported development build for this feature

**Project Type**: Tauri mobile application with a pure Rust workspace core and
application-owned infrastructure/inbound adapters

**Performance Goals**: Stream files up to the backend's 25,000,000-byte product
limit without loading the complete file into memory; throttle advisory progress
to at most 10 updates per second; recover an existing operation within one user
action after relaunch

**Constraints**: Foreground-only transfer; no provider or backend secret in the
client; no arbitrary URL or path accepted from the WebView; HTTPS-only production
transport; one terminal winner; no automatic retry after uncertain outcomes;
exact create replay is allowed only when a lost response also lost the backend ID

**Scale/Scope**: One active transcription per finalized source, at most three
backend non-terminal operations per user, recordings up to 10 minutes and
25,000,000 bytes, five named commands, one advisory event stream, and no
production recorder/UI integration

## Constitution Check

_GATE: Passed before Phase 0 and re-checked after Phase 1 design._

- **Mobile first — PASS**: iOS and Android share product semantics; foreground,
  offline, termination, slow-network, and physical-device fixture validation are
  explicit. No new microphone permission, background transfer, realtime work, or
  desktop release is introduced.
- **Hexagonal Rust — PASS**: `transcription-core` contains domain/application and
  outbound ports only. Tauri, HTTP, files, tokens, persistence, and events remain
  in root application adapters. Thin commands delegate to one service authority.
- **Feature-Sliced React — PASS**: Issue #5 adds no React state. Contracts reserve
  remote operation ownership for TanStack Query in #6 and prohibit Zustand
  duplication.
- **Secure transcription — PASS**: Mobile calls only the application backend;
  authorization is acquired immediately before dispatch and never persisted.
  Audio is streamed from trusted app-private storage and sensitive fields are
  absent from events, errors, and logs.
- **Resilient voice flow — PASS**: Stable identities, atomic intent persistence,
  compare-and-swap terminal ownership, exact idempotent replay, retry guidance,
  cancellation races, restart recovery, automated tests, and physical-device
  scenarios are designed.

### Post-design re-check

All five gates remain PASS. The design discovered one Issue #3 contract
limitation: a lost create response also loses the server operation ID. The
resolved behavior conforms to the existing contract by replaying the exact
multipart create request with the same idempotency key and fingerprint only in
that case. No new endpoint is invented, and provider dispatch remains deduplicated.

## Project Structure

### Documentation (this feature)

```text
specs/004-transcription-upload-usecase/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── transcription-ports.md
│   └── tauri-commands.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
src-tauri/
├── crates/
│   ├── recorder-core/
│   └── transcription-core/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── domain.rs
│       │   ├── ports.rs
│       │   └── application.rs
│       └── tests/
│           ├── state_machine.rs
│           ├── use_case.rs
│           └── recovery.rs
├── src/
│   ├── inbound/
│   │   ├── mod.rs
│   │   └── transcription_commands.rs
│   ├── infrastructure/
│   │   ├── mod.rs
│   │   └── transcription/
│   │       ├── mod.rs
│   │       ├── auth_session.rs
│   │       ├── http_backend.rs
│   │       ├── local_operation_store.rs
│   │       ├── private_source_audio.rs
│   │       └── tauri_event_sink.rs
│   ├── lib.rs
│   └── transcription_state.rs
├── tests/
│   └── transcription_http_contract.rs
└── capabilities/
    └── default.json

tests/device/
└── transcription-upload-usecase.md
```

**Structure Decision**: Follow the existing `recorder-core` compile-isolated
pattern for pure domain/application contracts. Keep HTTP, source files,
persistence, auth, event emission, and Tauri commands in the root application as
adapters and composition. Do not create a native plugin because transcription
transport does not require a platform-specific native seam. Do not add React
slices until Issue #6 owns the UI integration and asynchronous state.

## Complexity Tracking

No Constitution violations require an exception.
