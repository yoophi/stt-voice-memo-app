# Data Model: Transcription Upload Use Case

## Aggregate boundary

`TranscriptionOperation` is the aggregate root. It owns the stable client
identity, immutable source/options fingerprint, phase, first terminal winner,
retry metadata, cleanup disposition, event sequence, and persistence revision.
Adapters may project backend and source records into it, but cannot decide state
transitions.

## Identifiers and value objects

| Type                       | Rules                                                                                                                             |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `TranscriptionOperationId` | Client-generated canonical lowercase UUID; stable from local intent through cleanup; used as the 20–128 character idempotency key |
| `BackendOperationId`       | Opaque non-blank server identity learned from a create response; never synthesized locally                                        |
| `SourceAudioId`            | Opaque non-blank identity for exactly one finalized source artifact                                                               |
| `SubmissionFingerprint`    | Lowercase 64-character SHA-256 over immutable source integrity plus normalized options and contract version                       |
| `BackendRequestId`         | Optional opaque content-free support correlation; not an operation identity                                                       |
| `Revision`                 | Monotonic unsigned integer used for repository compare-and-swap                                                                   |

## SourceDescriptor

| Field           | Validation                                                         |
| --------------- | ------------------------------------------------------------------ |
| `sourceAudioId` | Matches the requested trusted source identity                      |
| `mediaType`     | One of the product formats defined by the backend contract         |
| `fileExtension` | Consistent with verified media type                                |
| `byteLength`    | `1..=25_000_000`                                                   |
| `durationMs`    | `1..=600_000`                                                      |
| `sha256`        | Lowercase 64-character SHA-256 matching the current file           |
| `languageHint`  | Absent or normalized BCP 47 value accepted by the backend contract |

The adapter-private locator is never stored in the aggregate or returned through
IPC. The source adapter revalidates identity, containment, size, and integrity
immediately before every streamed request.

## TranscriptionOperation

| Field                | Rule                                                                             |
| -------------------- | -------------------------------------------------------------------------------- |
| `id`                 | Stable client operation and idempotency identity                                 |
| `sourceAudioId`      | Immutable after creation                                                         |
| `fingerprint`        | Immutable after creation; changed content/options are rejected                   |
| `backendOperationId` | Absent until a create/replay response is observed                                |
| `phase`              | One canonical phase from the transition table                                    |
| `attempt`            | Increments only when a new network attempt begins                                |
| `progress`           | Optional attempt-scoped supplied/total byte count and sequence                   |
| `terminalWinner`     | `completed`, `cancelled`, `terminalFailure`, or absent; first value is immutable |
| `failure`            | Optional stable content-free failure for failure/uncertain phases                |
| `retry`              | Optional earliest retry time and bounded-attempt metadata                        |
| `cleanup`            | Remote content availability/deletion disposition                                 |
| `backendRequestId`   | Optional safe support correlation from the latest response                       |
| `eventSequence`      | Monotonic sequence for advisory events                                           |
| `revision`           | Repository compare-and-swap version                                              |
| `cancelRequested`    | Durable intent set before stopping transfer or requesting remote deletion        |

Transcript text is returned only in an authorized completed command result. It is
not persisted in the operation record; Issue #7 owns durable transcript/memo text.

## Operation phases

```text
ready
  -> waiting_for_network
  -> waiting_for_authorization
  -> uploading
  -> queued
  -> processing
  -> completed
  -> retryable_failure
  -> uncertain
  -> terminal_failure
  -> cancelling
  -> cancelled
  -> cleanup_pending
```

### Transition rules

| From                                     | Event                                      | To / outcome                                               |
| ---------------------------------------- | ------------------------------------------ | ---------------------------------------------------------- |
| none                                     | create local intent                        | `ready`, persisted before network                          |
| `ready`, waiting, or `retryable_failure` | submit/retry while online and authorized   | `uploading`, same IDs, increment attempt                   |
| `ready`, `waiting_for_network`           | submit while offline                       | `waiting_for_network`, no network call                     |
| non-terminal                             | authorization unavailable/expired          | `waiting_for_authorization`; explicit retry after refresh  |
| `uploading`                              | create accepted                            | backend `queued` or `processing`; store backend ID         |
| `uploading` without backend ID           | response lost/timeout                      | `uncertain`; exact create replay is permitted              |
| non-terminal with backend ID             | status queued/processing                   | `queued` / `processing`                                    |
| non-terminal                             | completed with valid text                  | `completed`, winner set once                               |
| non-terminal                             | retryable failure                          | `retryable_failure`, retry guidance stored                 |
| non-terminal                             | uncertain failure                          | `uncertain`, no blind automatic POST                       |
| non-terminal                             | terminal/user-actionable/malformed failure | `terminal_failure`, winner set once                        |
| local-only non-uploading                 | cancel                                     | `cancelled`, winner set; no remote call                    |
| `uploading` without backend ID           | cancel                                     | persist intent, stop transfer, then reconcile exact replay |
| remote non-terminal                      | cancel intent                              | `cancelling`, intent persisted before DELETE               |
| `cancelling`                             | DELETE confirmed cancelled/deleted/204     | `cancelled`; cleanup completed or pending                  |
| `cancelling`                             | DELETE outcome unknown                     | `cleanup_pending`; retain cancel intent and reconcile      |
| any terminal                             | later conflicting terminal event           | stored winner returned; event ignored or conflict reported |

## Failure

| Field          | Rule                                                                                                                            |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `code`         | Stable product/backend contract code; unknown wire codes become malformed-response failure                                      |
| `category`     | `retryable`, `userActionable`, `terminal`, or `uncertain`                                                                       |
| `retryable`    | Must agree with category and code catalog                                                                                       |
| `retryAfterMs` | Optional non-negative guidance; required for temporary rate limit and retryable unavailable responses when supplied by contract |
| `requestId`    | Optional safe backend correlation                                                                                               |

Raw transport messages, submitted values, provider payloads, paths, tokens,
audio, and transcript text are not fields.

## FinalTranscript

| Field                | Rule                                                            |
| -------------------- | --------------------------------------------------------------- |
| `operationId`        | Matches the completed local operation                           |
| `backendOperationId` | Matches the resolved server operation                           |
| `text`               | Trimmed, non-blank final text; never included in events or logs |
| `language`           | Optional normalized provider-neutral language                   |

## UploadObservation

| Field           | Rule                                      |
| --------------- | ----------------------------------------- |
| `operationId`   | Stable local operation identity           |
| `attempt`       | Must equal the current aggregate attempt  |
| `sequence`      | Strictly increases within the attempt     |
| `suppliedBytes` | Monotonic and no greater than total bytes |
| `totalBytes`    | Positive immutable source byte length     |

Observations received for an older attempt, with a non-increasing sequence or
byte count, or after a terminal winner are ignored.

## CleanupDisposition

- `notScheduled`: remote content is still governed by active processing.
- `scheduled`: backend accepted cancellation/deletion and supplied a deadline.
- `inProgress`: cleanup remains underway.
- `failedRetrying`: cleanup failed but remains retryable before the deadline.
- `completed`: sensitive remote content is unavailable and cleanup is complete.

Cleanup state never erases the terminal winner.

## PersistedOperationRecord

Persist only the aggregate fields required for recovery. Exclude transcript text,
source URI/path, audio bytes, token, authorization header, provider body, signed
URL, and raw error. Each record includes a schema version and revision. A write
must compare the expected revision, serialize to a sibling temporary file, sync,
rename, and sync the parent directory before success is acknowledged.

## Relaunch recovery projection

- `ready` / `waiting_for_network`: return recoverable state; do not auto-submit.
- `uploading` without backend ID: project to `uncertain`; require explicit exact
  replay.
- `queued` / `processing`: status may be resolved using the backend ID.
- `retryable_failure`: honor retry time and require explicit retry in this feature.
- `uncertain` with backend ID: GET status; without it, exact replay only.
- `cancelling` / `cleanup_pending`: repeat idempotent DELETE or GET reconciliation.
- Terminal states remain immutable and are returned without network side effects.
