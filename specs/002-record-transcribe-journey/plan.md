# Implementation Plan: Record and Transcribe Memo Journey Contract

**Branch**: `main` | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-record-transcribe-journey/spec.md`

## Summary

Define the first complete mobile recording-to-memo journey as a contract package
consumed by Issues #3 through #7. The package establishes prioritized scenarios,
canonical state transitions, stable identities and idempotency, foreground mobile
lifecycle behavior, backend-mediated OpenAI security, audio/transcript retention,
and future physical-device validation. Issue #2 remains documentation-only and
does not expose fake controls or preempt the native, backend, Rust, integration,
or UI work assigned to follow-up issues.

## Technical Context

**Language/Version**: Markdown contract artifacts; future implementation baseline
is Rust stable edition 2024 and TypeScript 5 / React 19

**Primary Dependencies**: Project constitution, Tauri 2 mobile lifecycle and
permission contracts, application-controlled backend, OpenAI Audio Transcriptions
API behind that backend

**Storage**: No runtime storage added in Issue #2; contract requires future
app-private durable journey/source storage and local memo persistence

**Testing**: Specification checklist, cross-artifact traceability analysis, and
future Rust contract tests, React behavior tests, backend contract tests, and
physical iOS/Android device matrix defined by this plan

**Target Platform**: iOS 15+ and Android API 24+ physical devices; desktop,
background recording, and realtime transcription excluded

**Project Type**: Mobile-first Tauri application; this feature is its shared
behavior/architecture contract package

**Performance Goals**: User actions acknowledged within one second; no duplicate
operation in 100 repeated-action trials; backend hard upload/processing limits are
selected in Issue #3

**Constraints**: Foreground capture only, explicit send, no client OpenAI secret
or provider model, privacy-first audio deletion, stable idempotency across retry
and relaunch, no production runtime changes in Issue #2

**Scale/Scope**: Three prioritized user journeys, fifteen canonical states, five
domain entities, three contracts, one device-validation matrix, five dependent
implementation issues

## Constitution Check

_GATE: PASS before Phase 0; PASS after Phase 1 design._

- **Mobile first — PASS**: iOS and Android permission, actual background entry,
  interruptions, route changes, termination, touch accessibility, and physical
  device scenarios are distinct where needed. Background, realtime, and desktop
  recording are excluded.
- **Hexagonal Rust — PASS**: The data model assigns transitions to a future pure
  journey aggregate and defines recorder/transcription/persistence as ports.
  Tauri, native APIs, filesystem, HTTP, backend, and OpenAI remain adapters.
- **Feature-Sliced React — PASS**: The architecture impact and data model assign
  page/widget composition, feature actions, entity APIs, and shared primitives.
  Zustand owns live capture UI only; TanStack Query owns durable/remote async
  state.
- **Secure transcription — PASS**: Audio leaves the device only after explicit
  action through an authenticated application backend. Provider credentials and
  model choice remain server-side. Creation, transfer, retention, cleanup, and
  user deletion are explicit.
- **Resilient voice flow — PASS**: Canonical states cover permission, capture,
  finalization, offline queue, upload, processing, edit, save, cancellation,
  duplicate action, late result, partial failure, and relaunch. Stable IDs and
  backend-owned idempotency prevent duplicate operations.

**Post-design re-check**: `data-model.md`, all three contracts, and
`quickstart.md` preserve the five gates. The design introduces no capability,
secret, state-store duplication, or platform dependency. No exception approval
is required.

## Project Structure

### Documentation (this feature)

```text
specs/002-record-transcribe-journey/
├── checklists/
│   └── requirements.md
├── contracts/
│   ├── journey-state-machine.md
│   ├── recorder-port.md
│   └── transcription-boundary.md
├── data-model.md
├── plan.md
├── quickstart.md
├── research.md
├── spec.md
└── tasks.md
```

### Future source code ownership (Issues #3 through #7)

```text
src/
├── app/                              # providers and routing only
├── pages/record-memo/
├── widgets/record-transcribe-workspace/
├── features/
│   ├── record-memo/
│   ├── transcribe-recording/
│   ├── edit-transcript/
│   ├── save-memo/
│   └── manage-audio-retention/
├── entities/
│   ├── recording/
│   ├── transcription/
│   └── memo/
└── shared/
    ├── api/tauri.ts
    └── ui/

src-tauri/src/
├── domain/
│   ├── journey.rs
│   ├── recording.rs
│   ├── transcription.rs
│   └── memo.rs
├── ports/
│   ├── recorder.rs
│   ├── transcription.rs
│   ├── memo_repository.rs
│   ├── journey_repository.rs
│   └── source_audio_store.rs
├── application/
│   ├── record_memo.rs
│   ├── transcribe_memo.rs
│   ├── save_memo.rs
│   └── recover_journey.rs
├── inbound/tauri_commands/record_transcribe.rs
└── infrastructure/
    ├── recorder/
    ├── backend_transcription_client.rs
    ├── local_journey_repository.rs
    ├── local_memo_repository.rs
    └── source_audio_store.rs

tests/
├── contract/
├── integration/
└── device/
```

**Structure Decision**: Issue #2 writes only the documentation tree. The future
paths show one intended ownership location so later issues do not invent
competing workflow state. No placeholder modules, empty Zustand store, mock
production adapter, microphone capability, or workspace crate is created now.

## Implementation Strategy

1. Complete the prioritized spec and requirements checklist from Issue #2.
2. Record primary-source research and explicit alternatives for lifecycle,
   permission, provider, idempotency, retention, and architecture choices.
3. Define stable identities/entities and the complete journey state machine.
4. Define native recorder and backend-mediated transcription semantics without
   selecting their implementation or wire schemas prematurely.
5. Trace requirements to contracts and define the physical-device matrix used by
   follow-up work.
6. Analyze spec, plan, model, contracts, quickstart, and tasks for coverage,
   consistency, constitution compliance, and scope leakage before handoff.

## Complexity Tracking

Constitution violations or justified exceptions: none.
