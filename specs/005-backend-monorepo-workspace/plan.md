# Implementation Plan: Backend Monorepo Workspace

**Branch**: `011-backend-monorepo-workspace` | **Date**: 2026-08-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-backend-monorepo-workspace/spec.md`

## Summary

Establish one root pnpm workspace and one root Cargo workspace while leaving the
mobile React application and `src-tauri` directory at their existing locations.
Add an explicitly non-runtime backend workspace reservation, make the existing
OpenAPI file a named workspace package and reproducible contract source, expose
scoped root commands, enforce import/configuration/secret boundaries, and add a
tested path classifier that drives mobile, backend, contract, and full CI jobs.
This incremental layout avoids moving generated Apple/Android projects and keeps
Issue #12 free to implement the first backend application module.

## Technical Context

**Language/Version**: Node.js 22.22+ with ECMAScript modules, TypeScript 5.9 for the existing mobile app, Rust stable 1.85+ edition 2024, existing Swift package tests

**Primary Dependencies**: pnpm 11 workspace/filtering, Cargo virtual workspace resolver 3, existing Vite/Vitest/ESLint/Prettier/Tauri 2 toolchain, GitHub Actions; no new backend runtime framework or provider SDK

**Storage**: N/A; tracked manifests and deterministic generated contract metadata only

**Testing**: Vitest contract tests for workspace boundaries/path selection/secret scanning/drift, existing frontend Vitest tests, Cargo workspace tests and Clippy, Swift Package tests, Tauri mobile build/project checks

**Target Platform**: Repository tooling on macOS and Linux CI; unchanged Tauri iOS 15+ and Android API 24+ targets

**Project Type**: Mobile application plus reserved backend application in a pnpm and Cargo monorepo

**Performance Goals**: Path classification completes in under 2 seconds for 10,000 changed paths; scoped checks avoid unrelated native jobs; clean contract generation is byte-for-byte reproducible

**Constraints**: Keep `src-tauri/gen/apple` and `src-tauri/gen/android` discovery stable; no production backend runtime; no real secret; no duplicated OpenAPI source; root commands must distinguish unavailable from passed

**Scale/Scope**: One existing mobile package, one backend workspace reservation, one contract package, four CI scopes, five Rust workspace members after future backend modules are added

## Constitution Check

_GATE: Passed before Phase 0 and re-checked after Phase 1._

- **Mobile first — PASS**: The mobile source, `src-tauri`, generated Apple/Android
  paths, platform targets, permissions, and foreground lifecycle behavior remain
  unchanged. Automated path/build checks are included; physical-device launch
  evidence is excluded from this implementation PR and owned by follow-up Issue
  #23. Desktop/background/realtime work is excluded.
- **Hexagonal Rust — PASS**: The root Cargo workspace only changes ownership and
  build orchestration. Existing pure crates stay isolated, and the reserved
  backend map explicitly separates domain/application, ports, inbound, and
  infrastructure without selecting an adapter dependency.
- **Feature-Sliced React — PASS**: No React slice or state is added or moved.
  TanStack Query and Zustand ownership remain unchanged; workspace tooling owns
  no product state.
- **Secure transcription — PASS**: The canonical contract contains no provider
  credential. Backend-only configuration is separately named, ignored, and
  checked against mobile source/build output with synthetic canaries. This
  feature handles no audio or transcript content.
- **Resilient voice flow — PASS**: Existing recorder/transcription tests remain
  in the full validation path. No state transition changes; physical iOS/Android
  launch regression remains a documented release gate owned by Issue #23 rather
  than this workspace implementation PR.

### Post-Design Re-check

All five gates remain satisfied. The contracts define explicit ownership rather
than a shared business-logic module, and the physical-device evidence is neither
conflated with automated CI nor claimed by this PR. Follow-up Issue #23 owns that
release gate. No constitutional exception is required.

## Project Structure

### Documentation (this feature)

```text
specs/005-backend-monorepo-workspace/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── workspace-boundaries.md
│   ├── root-commands.md
│   ├── contract-generation.md
│   ├── configuration-boundary.md
│   └── ci-path-matrix.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml                              # virtual Rust workspace root
Cargo.lock                              # single Rust lockfile
package.json                            # mobile package and root command facade
pnpm-workspace.yaml                     # pnpm workspace declaration

apps/
└── backend/
    ├── package.json                    # scoped command facade; no runtime dependency
    ├── .env.example                    # backend-only names and safe placeholders
    └── README.md                       # target hexagonal module map and ownership

contracts/
├── package.json                        # @stt-voice-memo/contracts workspace package
└── transcription-api/v1/
    ├── openapi.json                    # only canonical wire source
    └── generated/contract-manifest.json

src/                                    # unchanged React FSD mobile source
src-tauri/                              # unchanged mobile/Tauri package root
├── Cargo.toml                          # root-workspace member package
├── crates/{recorder-core,transcription-core}/
├── plugins/recorder/
└── gen/{apple,android}/

scripts/workspace/
├── check-boundaries.mjs
├── check-client-secrets.mjs
├── contract-artifacts.mjs
├── select-scopes.mjs
└── workspace-contract.test.mjs

.github/workflows/
└── validate.yml                        # scope selection and conditional jobs

docs/
└── monorepo-workspace.md               # contributor and ownership guide

tests/device/
└── backend-monorepo-workspace.md       # iOS/Android migration evidence
```

**Structure Decision**: Keep the existing mobile package at the repository root
so Tauri's `../dist`, generated native project paths, previous contract tests,
and developer commands do not require a risky mass move. Add `apps/backend` as a
reserved pnpm workspace package and future Rust module owner. Promote Cargo to a
virtual repository-root workspace so `src-tauri` and later backend crates share
one lockfile and scoped/full commands. Package the canonical contract in place,
without copying its OpenAPI source.

## Complexity Tracking

No constitution violations or additional runtime technologies are introduced.
