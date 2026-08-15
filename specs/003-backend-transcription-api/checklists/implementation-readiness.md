# Implementation Readiness: Backend Transcription API Contract

**Feature**: Issue #3 — Backend transcription API contract

**Canonical contract**: `contracts/transcription-api/v1/openapi.json`

**Evidence boundary**: This checklist proves contract completeness only. Runtime,
provider-dispatch, timing, concurrency, storage deletion, and physical-device
claims remain downstream conformance evidence.

## Acceptance and requirement mapping

| Scope                               | Contract evidence                                                                                                                                                     | Downstream conformance owner                                                      |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| US1 / FR-001–FR-007 / FR-012        | Versioned Bearer-authenticated POST/GET, multipart identity, seven states, async 202/Location, final-only result, provider-neutral fields, request IDs                | Backend handler/provider adapter; Issue #6 mobile integration                     |
| US2 / FR-008–FR-011 / FR-013–FR-015 | Owner-scoped fingerprint policy, replay/conflict responses, four problem categories, typed failed state, DELETE lifecycle, late-result discard, 120/600 second limits | Backend idempotency store, queue, cleanup worker, concurrency tests               |
| US3 / AUR-001–AUR-005               | Auth-derived ownership, indistinguishable 404, create/active/management/daily limits, named rejection examples                                                        | Auth middleware, rate/concurrency/usage stores, handler conformance tests         |
| MLR-001–MLR-005                     | Platform-neutral IDs/audio multipart, durable operation resource, uncertain outcome recovery, no native paths/APIs                                                    | Issues #4 and #6 on physical iOS/Android; slow/offline network tests              |
| PDL-001–PDL-004                     | No client provider/model/credential, excluded logging fields, 24-hour terminal cleanup, seven-day non-content tombstone, deleting state                               | Backend deployment review, provider data-control review, deletion jobs and audits |
| SC-001 / SC-003 / SC-005 / SC-008   | Nine no-network contract tests cover paths, local refs, success/states, conflicts, all 17 problems, privacy policy, and forbidden strings                             | None beyond normal regression maintenance                                         |
| SC-002 / SC-004 / SC-006 / SC-007   | Normative idempotency, pre-dispatch, timeout, deletion, and retention policies are machine-readable                                                                   | Backend load/concurrency, authorization, timing, and lifecycle suites             |

## Security and lifecycle review

- [x] Public surface is limited to create, read, and delete under `/v1`.
- [x] Bearer identity is global; no client-supplied user ID is accepted.
- [x] Unknown and cross-owner resources both resolve to 404.
- [x] Provider name, model selection, provider endpoint, and credentials are absent.
- [x] Exactly one server-verified audio part, SHA-256, size, duration, and format
      rules are machine-readable.
- [x] Same-fingerprint replay, mismatch, concurrent reservation uncertainty, and
      seven-day tombstones are explicit.
- [x] Cancellation invalidates work, discards late output, and makes content
      unavailable while cleanup remains observable.
- [x] Every catalog code has a named status/category/retryability example.
- [x] Audio, transcript, authorization, credentials, provider bodies, and storage
      paths are excluded from default operational logging.
- [x] Rejected uploads are removed immediately and terminal content within 24 hours.

## Analysis remediation

- [x] `cancelled` was removed from failure categories and retained as an operation state.
- [x] `INTERNAL_ERROR` is `uncertain`, preventing blind automatic POST replay.
- [x] Runtime success criteria are mapped to downstream backend/mobile evidence
      instead of being claimed by this contract-only feature.

## Automated validation

- [x] Focused contract test — 1 file, 25 tests passed
- [x] Full Vitest suite — 2 files, 27 tests passed
- [x] ESLint — passed with no findings
- [x] Prettier check — all selected files conform
- [x] Frontend production build — TypeScript and Vite build passed
- [x] Rust tests — unit, binary, and doc-test targets passed; existing vendored
      `swift-rs` warnings only
- [x] Git whitespace check — passed

## Scope integrity

- [x] No production source, Tauri capability, generated mobile host, dependency
      manifest, or lockfile changed.
- [x] User-owned untracked `.wtp.yml` remains untouched.
- [x] No OpenAI call, network call, API key, backend token, or real audio fixture
      was required.

## PR review follow-up

- [x] POST 422 and 429 responses directly reference every same-status example.
- [x] A test-only public contract double proves 100 concurrent matching submits
      dispatch once, changed fingerprints do not redispatch, and auth/ownership/rate/
      concurrency/daily-usage rejections happen before provider dispatch.
- [x] DELETE 204 includes `X-Request-Id` and `Cache-Control` headers.
- [x] The shared `FailureTuple` `oneOf` constrains every code/status/category/
      retryable combination for both RFC 9457 problems and operation failures.
- [x] `failed_retrying` cleanup is represented by a linked DELETE example and a
      lifecycle assertion.
- [x] Active-operation accounting decrements the current count after completion;
      a regression test proves only one newly opened slot can be reused.
- [x] The contract double proves cancellation/delete idempotency, late provider
      result discard, cleanup failure retry, and completed cleanup behavior.
- [x] Every scheduled, in-progress, or retrying cleanup requires a 24-hour
      `delete_by` deadline, including cancelled and retry examples.
- [x] Every HTTP error response references a status-specific RFC 9457 schema whose
      body `status` is constrained to the response status.
- [x] Completed DELETE releases active capacity and exposes deleting/deleted state
      without waiting for a delayed provider result.
- [x] Contract-double operations and problems include every canonical required
      field, including request identity, timestamps, cleanup, links, and status.
- [x] Deferred queued work can be cancelled before provider dispatch and yields a
      canonical `cancelled` state without invoking the provider.
- [x] Terminal operation states require `cleanup.delete_by`, including completed
      cleanup, and deleted idempotency replays return terminal HTTP 200.
- [x] Processing-state GET returns its current representation immediately without
      waiting for provider completion.
- [x] Provider rejection becomes a provider-neutral typed `failed` operation that
      is visible through GET and same-key terminal replay without leaking raw errors.
- [x] Create and combined GET/DELETE limits use clock-driven rolling windows,
      expire after 60 seconds, and include `Retry-After` guidance on rejection.
- [x] POST, GET, and DELETE reject missing authentication with the same canonical
      401 problem before rate-limit, ownership, provider, or cleanup work.
- [x] Active GET returns immediately with `Retry-After: 2`; terminal GET omits
      polling guidance.
