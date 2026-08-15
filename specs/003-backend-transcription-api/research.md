# Research: Backend Transcription API Contract

## Decision 1: Deliver a contract, not a backend service

**Decision**: Issue #3 commits one canonical OpenAPI 3.1.1 JSON document and
no-network contract tests. It adds no server framework, datastore, queue, auth
provider, OpenAI SDK, deployment, Tauri command, or mobile UI.

**Rationale**: The issue acceptance criteria require a versioned contract and
tests without OpenAI. Backend runtime choices are absent from the project and
would materially expand scope; Issues #5 and #6 consume this boundary later.

**Alternatives considered**: A mock production endpoint or speculative backend
framework was rejected because neither would establish real deployment readiness
and both would prematurely constrain architecture.

## Decision 2: Use OpenAPI 3.1.1 JSON

**Decision**: Publish `contracts/transcription-api/v1/openapi.json` using OpenAPI
3.1.1 and JSON Schema 2020-12 semantics. Keep human-readable semantic matrices in
this feature directory.

**Rationale**: JSON is a valid OpenAPI representation, parses with Node built-ins,
and avoids adding a YAML/OpenAPI parser for a contract-only feature. The versioned
root path gives backend and client work one canonical source.

**Alternatives considered**: YAML would require a direct parser for reliable
tests. Markdown-only contracts cannot mechanically detect broken references,
schema drift, or missing examples. A full lint CLI adds a large toolchain before
the repository has a backend build.

**Primary source**: [OpenAPI 3.1.1 specification](https://spec.openapis.org/oas/v3.1.1.html)

## Decision 3: Model an asynchronous operation resource

**Decision**:

- `POST /v1/transcriptions` accepts authenticated multipart audio and returns
  `202 Accepted`, `Location`, `Retry-After: 2`, `Cache-Control: no-store`, and the
  canonical operation.
- `GET /v1/transcriptions/{operationId}` returns current state/result.
- `DELETE /v1/transcriptions/{operationId}` is idempotent cancel/delete intent;
  it returns `202` while cleanup is pending and `204` once content is gone.
- A replay returns `202` for active operations or `200` for terminal operations.

**Rationale**: File upload and provider processing exceed reliable mobile request
windows. HTTP defines 202 as accepted but incomplete and recommends a status
monitor in its representation. A resource-based API supports relaunch recovery,
status resolution before retry, cancellation, and deletion without SSE/realtime.

**Alternatives considered**: A synchronous endpoint was rejected because an
uncertain timeout cannot distinguish rejection from accepted provider work.
Separate cancellation and deletion actions were rejected because one idempotent
DELETE intent covers both lifecycle stages.

**Primary sources**:

- [RFC 9110: 202 Accepted](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.3.3)
- [RFC 9110: DELETE](https://www.rfc-editor.org/rfc/rfc9110.html#section-9.3.5)
- [RFC 7578: multipart/form-data](https://www.rfc-editor.org/rfc/rfc7578.html)

## Decision 4: Product allowlist and provider isolation

**Decision**: The product accepts `m4a`, `mp3`, `mp4`, `mpeg`, `mpga`, `wav`, and
`webm`, up to 25,000,000 bytes and 10 minutes. The server verifies actual content,
recomputes SHA-256, and chooses provider model/options. The client may provide an
optional BCP 47 language hint but never a provider/model name. Provider response
is normalized to one final text field.

**Rationale**: Current OpenAI file transcription guidance recommends
`gpt-transcribe`, documents a 25 MB limit, and lists those seven formats. The API
reference has a slightly wider list, so an explicit conservative product
allowlist prevents provider drift from changing the public contract. The 10-minute
product cap bounds cost above the two-minute UX benchmark.

**Alternatives considered**: Direct provider enum exposure would couple mobile
releases to changing models. Supporting every provider format or chunking files
was rejected for the initial recorder-produced memo scope.

**Official OpenAI documentation**:

- [File transcription guide](https://developers.openai.com/api/docs/guides/speech-to-text)
- [Create transcription reference](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)
- [GPT Transcribe model](https://developers.openai.com/api/docs/models/gpt-transcribe)

## Decision 5: Backend-owned idempotency

**Decision**: Require an opaque 20–128 character `Idempotency-Key`. Scope it to
the authenticated owner and endpoint, and bind it to a server-computed fingerprint
of verified audio SHA-256 plus normalized language hint. Same key/fingerprint
returns the existing operation; changed fingerprint returns 422; an unresolved
simultaneous reservation conflict returns 409. Non-content tombstones remain
seven days after terminal outcome.

**Rationale**: OpenAI documents no idempotency guarantee for audio transcription;
provider request IDs are correlation only. Backend ownership prevents duplicate
provider cost across mobile timeout and relaunch.

**Alternatives considered**: Blind provider retry and client-only deduplication
were rejected because neither can prove single dispatch. The expired IETF draft
is design guidance, not a normative standard.

**Sources**:

- [IETF Idempotency-Key draft 07](https://datatracker.ietf.org/doc/draft-ietf-httpapi-idempotency-key-header/07/)
- [OpenAI API overview and request IDs](https://developers.openai.com/api/reference/overview)

## Decision 6: Normalize all errors with Problem Details

**Decision**: Every JSON error uses `application/problem+json` with RFC 9457
fields plus stable `code`, `category`, `retryable`, `request_id`, optional
`retry_after_seconds`, and optional safe field errors. Categories are
`retryable`, `user_actionable`, `terminal`, and `uncertain`; cancellation is a
resource state rather than an error category.

**Rationale**: Stable product codes isolate clients from provider/raw framework
errors. Categories directly support Issue #2 recovery UI. `detail` remains human
text and is not parsed.

**Alternatives considered**: Per-endpoint ad hoc errors and raw upstream payloads
were rejected for client coupling and leakage risk.

**Primary source**: [RFC 9457: Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457.html)

## Decision 7: Explicit status, retry, and limit semantics

**Decision**:

- 400 malformed request; 401 missing/invalid auth; 403 no entitlement; 404
  unknown or other-owner operation; 409 in-progress reservation/state conflict;
  410 known purged content; 413 too large; 415 unsupported/mismatched media; 422
  semantic validation or idempotency fingerprint mismatch; 429 rate/usage limit;
  500 internal; 503 temporary provider/backend unavailable; 504 uncertain timeout.
- Automatic retry applies only when category is retryable and honors
  integer-seconds `Retry-After` plus bounded backoff/jitter. An uncertain outcome
  requires GET resolution before any create retry.
- Create limit: 10/minute and 3 concurrent non-terminal operations per user.
  Management limit: 60/minute. Daily usage policy is checked pre-dispatch.

**Rationale**: Exact status/category/action pairs make clients deterministic and
prevent unsafe automatic retries. Limits bound abuse and provider spend.

**Primary sources**:

- [RFC 9110: HTTP status semantics](https://www.rfc-editor.org/rfc/rfc9110.html#section-15)
- [RFC 6585: 429 Too Many Requests](https://www.rfc-editor.org/rfc/rfc6585.html#section-4)
- [RFC 9110: Retry-After](https://www.rfc-editor.org/rfc/rfc9110.html#section-10.2.3)
- [OpenAI rate-limit guidance](https://developers.openai.com/api/docs/guides/rate-limits)

## Decision 8: Privacy lifecycle is independent of provider defaults

**Decision**: Reject/abandon uploads immediately; delete app-controlled accepted
audio and result content on explicit DELETE or within 24 hours of completed,
cancelled, or terminal failure. Preserve only non-content idempotency/audit
metadata. Logs/traces/errors exclude audio, transcript, auth, multipart, signed
locations, keys, and raw upstream errors. Record provider data controls before
production deployment.

**Rationale**: Current OpenAI endpoint controls list no training, abuse-monitoring
retention, or application-state retention for audio transcription and show ZDR
eligibility, but those controls do not delete application-controlled copies and
may depend on account/region configuration.

**Alternative considered**: Relying on provider defaults was rejected because it
does not cover the app's upload, result, or telemetry copies.

**Official OpenAI documentation**: [Data controls](https://developers.openai.com/api/docs/guides/your-data#default-usage-policies-by-endpoint)

## Decision 9: Test the public artifact without new dependencies

**Decision**: Use one Vitest ESM test to parse the canonical JSON, resolve every
local `$ref`, assert path/method/header/schema/example semantics, and scan for
forbidden provider/secret fields. No network call or OpenAI fixture is permitted.

**Rationale**: The repository already uses `scripts/*.test.mjs` for contract
artifacts. Existing Vitest and Node built-ins provide deterministic validation
without a transitive parser or new lockfile surface.

**Alternatives considered**: A dedicated OpenAPI linter can be introduced later
with backend CI policy. Importing transitive AJV/YAML packages directly was
rejected because they are not declared project dependencies.
