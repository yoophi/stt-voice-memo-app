# Application Backend Workspace

This directory reserves the application-backend boundary for Issues #12–#20.
Issue #11 does not implement a server, HTTP handler, persistence, authentication,
audio storage, queue, OpenAI adapter, or deployment target.

## Runtime status

`pnpm dev:backend` intentionally returns `WORKSPACE_RUNTIME_UNAVAILABLE` with a
non-zero status. This prevents the scaffold from being mistaken for a runnable or
validated production backend. Build, test, lint, and format commands validate the
workspace boundary and canonical contract only until a runtime-owning issue adds
code.

## Target architecture

Future Rust modules belong under this area and join the repository-root Cargo
workspace:

```text
apps/backend/
├── crates/
│   └── <bounded-context>/
│       ├── src/domain/
│       ├── src/application/
│       └── src/ports/
├── src/inbound/          # HTTP/worker adapters selected by later ADRs
├── src/infrastructure/   # persistence/provider/queue/deployment adapters
├── tests/
├── .env.example
└── package.json
```

Domain and application modules expose product-level values and commands only.
They must not depend on HTTP framework, Bearer/multipart, database, queue,
filesystem, object storage, OpenAI/provider, Tauri, native recorder, or deployment
SDK types. External behavior enters through explicit ports and adapters.

## Configuration

`.env.example` is the only tracked backend environment template. It contains
names and synthetic placeholders only. Working `.env*` files remain ignored.
Backend-only names and values are forbidden from the mobile source and bundle.

See [the repository workspace guide](../../docs/monorepo-workspace.md) and the
[canonical OpenAPI source](../../contracts/transcription-api/v1/openapi.json).
