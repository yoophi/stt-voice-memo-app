# Monorepo Workspace Guide

This repository develops the mobile Tauri application and application backend
from one review and validation boundary without sharing runtime modules or
server-only configuration.

## Ownership map

| Area           | Location                                                   | Owner responsibility                                                          |
| -------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Mobile         | root React/config files, `src/`, `src-tauri/`              | React FSD UI, Tauri composition, mobile Rust crates/adapters, native recorder |
| Backend        | `apps/backend/`                                            | future backend domain/application, ports, inbound and infrastructure adapters |
| Contract       | `contracts/`                                               | canonical transcription OpenAPI and deterministic derived artifacts           |
| Shared tooling | root manifests, `scripts/workspace/`, `.github/workflows/` | workspace orchestration, boundaries, CI selection, content-safe validation    |
| Evidence       | `specs/`, `tests/device/`, `docs/`                         | requirements, decisions, reproducible and physical validation records         |

The mobile and backend areas may consume the canonical wire contract. They may
not import each other's runtime code. Shared tooling is not a place for shared
business logic.

## Install once

```sh
corepack enable
pnpm install --frozen-lockfile
cargo metadata --no-deps --format-version 1
```

The pnpm workspace contains the root mobile package,
`@stt-voice-memo/backend`, and `@stt-voice-memo/contracts`. The virtual Cargo
workspace at the repository root owns the Tauri package and existing Rust crates,
with one root `Cargo.lock`.

## Root workflows

| Scope    | Development        | Build                 | Test                 | Lint                 | Format                 |
| -------- | ------------------ | --------------------- | -------------------- | -------------------- | ---------------------- |
| Mobile   | `pnpm dev:mobile`  | `pnpm build:mobile`   | `pnpm test:mobile`   | `pnpm lint:mobile`   | `pnpm format:mobile`   |
| Backend  | `pnpm dev:backend` | `pnpm build:backend`  | `pnpm test:backend`  | `pnpm lint:backend`  | `pnpm format:backend`  |
| Contract | no runtime         | `pnpm build:contract` | `pnpm test:contract` | `pnpm lint:contract` | `pnpm format:contract` |
| Full     | mobile alias       | `pnpm build`          | `pnpm test`          | `pnpm lint`          | `pnpm format:check`    |

Use `pnpm validate:mobile`, `pnpm validate:backend`,
`pnpm validate:contract`, or `pnpm validate`. The backend development command is
deliberately unavailable until a later issue implements a runtime.

Existing `pnpm tauri ios ...` and `pnpm tauri android ...` commands remain at the
root as CLI facades. The iOS host is available; the Android host/build path is
not initialized and therefore cannot run yet. Issue #24 makes the existing
Android facade executable by adding the minimal reviewed host. Mobile source and
generated projects stay under `src-tauri`; this migration does not move signing
or SDK-local files.

## Canonical contract workflow

`contracts/transcription-api/v1/openapi.json` is the only authored transcription
wire source. Use:

```sh
pnpm generate:contract
pnpm check:contract-drift
```

Never copy the OpenAPI document into a consumer. Future generated types must be
deterministic, declare the canonical source digest, and join the drift check.

## Configuration boundary

- Mobile-public settings require an explicitly reviewed client-safe namespace.
- Backend-only names are declared in `apps/backend/.env.example`.
- Working backend `.env*` files are ignored and supplied through local or CI
  secret stores.
- Provider credentials, backend signing secrets, user tokens, audio, and
  transcripts never belong in mobile bundles, repository fixtures, or evidence.

Run `pnpm verify:client-secret-boundary` to pass a unique synthetic canary
through an actual temporary Vite build and prove transformed output is rejected,
then run `pnpm check:client-secrets` against the normal client output. Neither
command reads arbitrary developer environment values, and the temporary canary
build is removed after validation.

## Add a backend domain module

1. Start from an owning issue and update its spec/plan before adding runtime code.
2. Add a compile-isolated crate below `apps/backend/crates/<context>` and register
   it in the root Cargo workspace.
3. Keep product entities/rules in `domain`, orchestration in `application`, and
   external contracts in `ports`.
4. Test the application interface with local adapters; do not inspect private
   state or introduce transport/provider types.
5. Add path classification and scoped/full validation coverage.

## Add an adapter

1. Confirm an existing port owns the external behavior; add a port only when the
   application needs a genuinely new seam.
2. Put HTTP, auth, storage, persistence, queue, provider, or deployment code in
   `inbound` or `infrastructure` under the backend owner.
3. Record a reviewed ADR before selecting a database, queue, auth provider,
   cloud/deployment target, HTTP framework, or OpenAI SDK.
4. Prove the adapter maps to product-level values and emits content-safe
   diagnostics.

## CI selection

`scripts/workspace/select-scopes.mjs` is the single path-classification authority
for local tests and GitHub Actions. Contract or root/shared changes select all
consumers; unknown and empty inputs fail safe to full validation. A manual full
workflow remains available regardless of path selection.

Automated builds do not replace physical-device evidence. PR #22 explicitly
excludes device execution; GitHub Issue
[#23](https://github.com/yoophi/stt-voice-memo-app/issues/23) owns the physical
iPhone/Android release gate and updates
`tests/device/backend-monorepo-workspace.md`. Its Android half depends on Issue
[#24](https://github.com/yoophi/stt-voice-memo-app/issues/24), which owns minimal
host initialization. Until both physical rows pass, the workspace implementation
may be code-complete but feature acceptance remains incomplete.
