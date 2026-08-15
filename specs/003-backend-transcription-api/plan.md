# Implementation Plan: Backend Transcription API Contract

**Branch**: `003-backend-transcription-api` | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-backend-transcription-api/spec.md`

## Summary

Define and mechanically validate a versioned OpenAPI 3.1.1 contract for an
application-controlled asynchronous transcription resource. The mobile client
submits one authenticated multipart recording with a stable idempotency key,
polls a canonical operation, and cancels or deletes it through the same resource.
The contract normalizes provider behavior into stable states and RFC 9457
problems, keeps provider/model/credentials server-side, bounds size, duration,
timeouts, rate and concurrency, and makes terminal content cleanup explicit.
Issue #3 delivers the contract and no-network tests, not a deployed backend.

## Technical Context

**Language/Version**: OpenAPI 3.1.1 JSON with JSON Schema 2020-12 semantics;
Node.js 22.22+ ECMAScript modules for contract tests

**Primary Dependencies**: Existing Vitest 4; Node built-in JSON/filesystem APIs;
no new runtime or development dependency

**Storage**: Contract only; future backend requires durable operation,
idempotency/tombstone, temporary audio, transcript, and cleanup records as
defined in `data-model.md`

**Testing**: Vitest no-network public-contract tests that parse the canonical
OpenAPI JSON, resolve local references, and verify paths, security, multipart,
states, error examples, idempotency, retention, and forbidden provider leakage

**Target Platform**: Backend-neutral HTTPS contract consumed identically by iOS
15+ and Android API 24+ clients

**Project Type**: Contract package inside a mobile-first Tauri repository;
production backend technology intentionally deferred

**Performance Goals**: upload acceptance or typed failure within 120 seconds;
95% terminal processing within 10 minutes; polling guidance of at least 2 seconds

**Constraints**: 25,000,000 bytes and 10 minutes maximum; direct multipart;
Bearer user auth; 10 creates/minute; 3 active operations/user; 60 management
requests/minute; terminal content deletion within 24 hours; seven-day non-content
idempotency tombstone; zero OpenAI calls in tests

**Scale/Scope**: Three versioned operations, seven resource states, three failure
four failure categories, seventeen documented error codes/examples, one
machine-readable contract and one focused contract test suite

## Constitution Check

_GATE: PASS before Phase 0; PASS again after Phase 1 design._

- **Mobile first — PASS**: One backend contract supports both mobile platforms,
  handles slow/uncertain mobile networking and relaunch by durable identity, and
  creates no recorder or background behavior requiring device verification in
  this feature.
- **Hexagonal Rust — PASS**: The contract is an external adapter boundary.
  Future domain/application logic consumes normalized operations/errors through a
  port and never depends on HTTP, multipart, OpenAPI, auth, storage, or OpenAI.
- **Feature-Sliced React — PASS**: Future entity APIs and TanStack Query own
  remote operation state. No React/Zustand implementation is added, and the plan
  explicitly prohibits mirroring durable server state into Zustand.
- **Secure transcription — PASS**: Bearer authentication, owner-scoped lookup,
  pre-dispatch validation/limits, backend-only OpenAI secrets/model, sensitive-log
  exclusions, and terminal deletion deadlines are normative contract elements.
- **Resilient voice flow — PASS**: Stable operation identity, request fingerprint,
  replay/conflict behavior, asynchronous states, retry categories, cancellation,
  deletion, late-result rejection, expiry, and no-network tests are explicit.

**Post-design re-check**: `data-model.md`, `contracts/http-api.md`,
`contracts/error-catalog.md`, and `quickstart.md` preserve all five principles.
No capability, provider credential, runtime dependency, or duplicate state owner
is introduced. No exception approval is required.

## Project Structure

### Documentation (this feature)

```text
specs/003-backend-transcription-api/
├── checklists/
│   ├── requirements.md
│   └── implementation-readiness.md
├── contracts/
│   ├── error-catalog.md
│   └── http-api.md
├── data-model.md
├── plan.md
├── quickstart.md
├── research.md
├── spec.md
└── tasks.md
```

### Implemented contract package

```text
contracts/
└── transcription-api/
    └── v1/
        └── openapi.json

scripts/
├── backend-transcription-api-contract.test.mjs
└── support/
    └── backend-transcription-contract-double.mjs
```

### Future source ownership (not implemented by Issue #3)

```text
src-tauri/src/
├── domain/transcription.rs
├── ports/transcription.rs
├── application/transcribe_memo.rs
└── infrastructure/backend_transcription_client.rs

src/entities/transcription/
├── api/
├── model/
└── index.ts
```

**Structure Decision**: Planning semantics stay with the feature artifacts;
the canonical machine-readable OpenAPI contract lives at a stable root contract
path so future backend, Rust adapter, client types, and CI share one source. A
focused `scripts/*.test.mjs` guard matches the existing repository test pattern
and avoids adding an OpenAPI/YAML toolchain. Production source remains unchanged.

## Implementation Strategy

1. Lock semantic resource, idempotency, security, error, and data-lifecycle
   behavior in design artifacts.
2. Write a public contract test that expects the canonical OpenAPI artifact and
   fails before it exists.
3. Add one minimal OpenAPI vertical slice for submit/status success and return to
   green.
4. Extend tests first for replay/conflict/retry/cancellation, then add matching
   contract paths, schemas, headers, and examples.
5. Extend tests first for auth/ownership/limits/privacy/cleanup and complete the
   contract without adding provider calls.
6. Run focused/full tests, format, lint, build, Rust regression, OpenAPI local-ref
   resolution, and sensitive fixture inspection.

## Complexity Tracking

Constitution violations or justified exceptions: none.
