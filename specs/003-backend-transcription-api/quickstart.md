# Validation Quickstart: Backend Transcription API Contract

## Scope

This validates the committed machine-readable contract without starting a
backend and without calling OpenAI. It proves Issue #3 contract readiness, not
runtime conformance of a future server.

## Prerequisites

- Node.js 22.22+
- pnpm 11 with dependencies installed from the existing lockfile
- No OpenAI API key, backend token, server, network, or audio recording required

## Focused validation

```bash
pnpm exec vitest run scripts/backend-transcription-api-contract.test.mjs
```

Expected: the contract parses, all local references resolve, and success,
recovery, security, privacy, and error-catalog assertions pass. The test-only
contract double also proves concurrent replay, conflict, authentication,
ownership, create-rate, active-operation, and daily-usage behavior without a
network or provider call.

## Full repository regression

```bash
pnpm exec vitest run
pnpm exec eslint .
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm exec prettier --check \
  contracts/transcription-api/v1/openapi.json \
  scripts/backend-transcription-api-contract.test.mjs \
  'specs/003-backend-transcription-api/**/*.md'
git diff --check
```

## Manual contract review

1. Confirm the only public paths are create, read, and delete under `/v1`.
2. Confirm every operation uses Bearer auth, no user ID comes from client input,
   and unknown/cross-owner identifiers share the same 404.
3. Confirm create is multipart, requires `Idempotency-Key` and audio SHA-256,
   returns 202 + Location + Retry-After, and exposes no provider model option.
4. Confirm active/terminal replay, fingerprint mismatch, concurrent conflict,
   uncertain timeout, cancellation, late result, deletion, and expiry all have
   deterministic outcomes.
5. Confirm every code in `contracts/error-catalog.md` has a named OpenAPI example
   with matching status/category/retry fields.
6. Search the contract artifact and test output for secret-looking values,
   unapproved transcript content, provider endpoints, storage paths,
   authorization tokens, and raw upstream errors. The only transcript fixture is
   the synthetic authorized completed-result example; negative test assertions
   may name forbidden patterns without containing their values.
7. Confirm `.wtp.yml`, production source, Tauri capabilities, mobile hosts, and
   dependency manifests are unchanged by Issue #3.

## Future conformance handoff

- Backend implementation must run these canonical examples against its HTTP
  handler with a fake provider boundary and prove one dispatch under concurrency.
- Issue #5 maps OpenAPI states/problems into the Rust transcription port without
  importing HTTP/provider types into domain/application.
- Issue #6 validates mobile timeout/relaunch/status/cancel/delete behavior against
  a conforming backend and adds physical-device network recovery evidence.
