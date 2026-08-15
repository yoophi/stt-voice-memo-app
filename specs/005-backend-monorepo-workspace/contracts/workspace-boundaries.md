# Workspace Boundary Contract

## Ownership map

| Area           | Owned roots                                                | May depend on                                  | Must not depend on                             |
| -------------- | ---------------------------------------------------------- | ---------------------------------------------- | ---------------------------------------------- |
| Mobile         | root React/config files, `src/`, `src-tauri/`              | canonical contract, mobile Rust ports/adapters | `apps/backend`, backend-only configuration     |
| Backend        | `apps/backend/` and future backend Cargo members           | canonical contract, its own ports/adapters     | `src/`, Tauri package/runtime, native recorder |
| Contract       | `contracts/`                                               | no runtime area                                | mobile/backend implementation types            |
| Shared tooling | root manifests, `scripts/workspace/`, `.github/workflows/` | area metadata only                             | product business logic or secrets              |

`docs/`, `specs/`, and `tests/device/` describe or validate these owners; they do
not become runtime dependency shortcuts.

## Rust boundary

- Root `Cargo.toml` is a virtual workspace and owns `Cargo.lock`, patches, and
  full/scoped Rust commands.
- `src-tauri/Cargo.toml` is the mobile Tauri package, not a nested workspace.
- Existing `recorder-core` and `transcription-core` keep their compile-isolated
  domain/application contracts.
- Future backend crates live below `apps/backend/` and enter the root workspace.
- Backend domain/application crates may not declare Tauri, HTTP framework,
  OpenAI/provider SDK, database, queue, filesystem, or deployment SDK dependencies.

## JavaScript boundary

- The repository root remains the mobile package and workspace command facade.
- `apps/backend` is a named private workspace package with no production runtime
  until a later issue selects and implements one.
- `contracts` is a named private workspace package whose exported source is the
  canonical OpenAPI file in place.
- Workspace-to-workspace imports use explicit package ownership; relative imports
  crossing runtime areas are forbidden.

## Automated enforcement

Boundary validation fails on:

1. mobile source/config importing or resolving into `apps/backend`;
2. backend files importing or resolving into `src`, `src-tauri`, mobile package
   entry points, or Tauri/native recorder modules;
3. any second file named as a canonical transcription OpenAPI source;
4. backend-only environment names in mobile source or built assets;
5. backend runtime dependencies added before an owning feature/ADR records them.
