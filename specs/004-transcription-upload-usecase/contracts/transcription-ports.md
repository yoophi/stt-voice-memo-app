# Contract: Transcription Core Ports

## Purpose

Define technology-neutral seams consumed by the transcription application use
case. Rust signatures are illustrative; domain meaning and invariants are
normative. Ports never expose Tauri, HTTP, filesystem paths, provider types, or
credentials to domain/application code.

## TranscriptionPort

Logical operations:

| Operation | Input                                                                                                                        | Result                                                |
| --------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `create`  | stable local operation/idempotency ID, trusted source identity/fingerprint, normalized options, attempt-scoped progress sink | accepted/replayed backend operation or stable failure |
| `get`     | local operation ID plus known backend operation ID                                                                           | canonical backend operation or stable failure         |
| `delete`  | local operation ID plus known backend operation ID                                                                           | cancelled/deleting/deleted outcome or stable failure  |

Invariants:

- `create` rebuilds a fresh streamed request per attempt; streamed requests are
  never cloned.
- An uncertain create without backend ID may replay only the exact immutable
  source/options fingerprint and idempotency key.
- `get` and `delete` cannot run without a backend ID.
- Wire/provider errors are normalized before crossing the port.
- No method logs or returns audio bytes, token, path, raw response, or provider
  model.

## SourceAudioPort

Logical operations:

- `inspect(sourceAudioId)` validates a trusted app-private record and returns a
  content-free `SourceDescriptor`.
- Infrastructure may open an adapter-private stream for `create`, but the locator
  is not part of a core DTO.
- Every upload attempt rechecks containment, readability, size, media metadata,
  and checksum. A changed source fails before network dispatch.

Issue #5 supplies a deterministic fixture/manifest adapter. Issue #6 owns
registering actual recorder artifacts.

## OperationRepository

Logical operations:

| Operation          | Rule                                                                                                    |
| ------------------ | ------------------------------------------------------------------------------------------------------- |
| `get_or_create`    | Atomically returns the existing source/options operation or persists one new intent before side effects |
| `load`             | Returns one content-free record by local operation ID                                                   |
| `compare_and_swap` | Writes only when expected revision matches; increments revision                                         |
| `list_unfinished`  | Returns non-terminal recovery records without transcript/audio content                                  |

Conflicting revisions are application reconciliation signals, not generic
storage failures. Storage failure before intent persistence forbids network
dispatch.

## AuthorizationPort

- Acquires an opaque user-scoped access token immediately before an HTTP request.
- Token values are non-serializable and redacted from `Debug`/`Display`.
- Authentication failure maps to a user-actionable state and stops automatic
  network work.
- The token never enters an operation record, event, query key, URL, or IPC DTO.

## ConnectivityPort and Clock

- Connectivity is an optimization for deciding `waiting_for_network`, not proof
  that the backend is reachable. The request result remains authoritative.
- Clock supplies deterministic current time for retry eligibility and tests.
- No production sleep occurs in the core; callers schedule explicit retry/status
  actions.

## OperationEventSink

- Receives advisory, content-free events after a state is durably committed.
- Event delivery failure does not roll back the committed aggregate.
- Commands/status remain authoritative; stale sequence/attempt projections are
  ignored by consumers.

## Application service

The single `TranscriptionService` exposes:

- `submit(sourceAudioId, options)`
- `status(operationId)`
- `retry(operationId)`
- `cancel(operationId)`
- `recover()`

It alone owns state transitions, first-terminal selection, retry eligibility,
exact replay permission, CAS reconciliation, and late-event rejection.
