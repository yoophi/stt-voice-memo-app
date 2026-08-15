# Data Model: Backend Transcription API Contract

## Scope

These are transport-neutral resource concepts and invariants. The OpenAPI
implementation serializes them, while a future backend selects persistence and
provider adapters. Audio and transcript text never appear in identifiers,
telemetry metadata, or problem details.

## TranscriptionOperation

| Field                    | Type / rule                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------- |
| `id`                     | Opaque server operation ID                                                         |
| `owner`                  | Derived from authenticated principal; never accepted from body                     |
| `sourceAudioId`          | Client domain identity, opaque and non-secret                                      |
| `idempotencyKey`         | 20–128 printable ASCII characters, unique within owner/create scope                |
| `fingerprint`            | Server-computed digest of verified decoded source integrity and normalized options |
| `state`                  | Canonical state below                                                              |
| `createdAt`, `updatedAt` | UTC timestamps                                                                     |
| `result`                 | Present only in `completed`; one non-blank final text                              |
| `failure`                | Present only in `failed`; typed category/code without raw provider data            |
| `cleanup`                | Content deletion state and deadline                                                |

### Canonical state transitions

```text
queued -> processing -> completed
   |          |       -> failed
   |          |       -> cancelled
   |          |       -> deleting -> deleted
   |          +----------^             ^
   +----------------------^-------------+
```

- `completed`, `failed`, and `cancelled` are processing-terminal.
- `deleting` and `deleted` describe content availability/cleanup after any
  processing-terminal state or explicit DELETE.
- Late provider output cannot move `cancelled`, `deleting`, or `deleted` back to
  `completed`.
- A GET may observe any committed transition, but never transcript text outside
  `completed` before deletion.

## SubmissionFingerprint

| Component           | Normalization                                            |
| ------------------- | -------------------------------------------------------- |
| Authenticated owner | Stable internal principal scope                          |
| Verified audio      | Server-computed SHA-256 after complete upload validation |
| Source audio ID     | Opaque client identity                                   |
| Language hint       | Lowercase normalized BCP 47 tag or absent marker         |
| Contract version    | `v1` to prevent future option changes from colliding     |

The client-supplied checksum is an integrity assertion, not authoritative. A
server mismatch is semantic validation failure. Same idempotency key with the
same fingerprint returns the original operation. A changed fingerprint never
mutates or re-dispatches it.

## TranscriptionResult

| Field      | Rule                                                    |
| ---------- | ------------------------------------------------------- |
| `text`     | Non-empty normalized final transcript                   |
| `language` | Optional normalized detected language; provider-neutral |

Provider model, prompt, raw response, log probabilities, timestamps, usage, and
request payload are not client result fields. Operational usage may be stored in
non-content audit metadata.

## ProblemResponse

| Field                 | Rule                                                        |
| --------------------- | ----------------------------------------------------------- |
| `type`                | Stable documentation URI for the product problem type       |
| `title`               | Stable short title                                          |
| `status`              | Matching HTTP status                                        |
| `detail`              | Safe human text; clients do not parse it                    |
| `instance`            | Optional safe occurrence URI                                |
| `code`                | Stable machine error code from `contracts/error-catalog.md` |
| `category`            | `retryable`, `user_actionable`, `terminal`, or `uncertain`  |
| `retryable`           | Boolean consistent with category/code                       |
| `request_id`          | Opaque backend correlation identity                         |
| `retry_after_seconds` | Optional non-negative integer matching Retry-After guidance |
| `errors`              | Optional field-name/reason items without submitted values   |

## CleanupRecord

| Field              | Rule                                                                           |
| ------------------ | ------------------------------------------------------------------------------ |
| `state`            | `not_scheduled`, `scheduled`, `in_progress`, `completed`, or `failed_retrying` |
| `deleteBy`         | Required after terminal outcome; no later than 24 hours                        |
| `contentAvailable` | False immediately after explicit DELETE or expiry                              |

Cleanup metadata contains no storage location. Failure remains retryable until
completed within the deadline and is observable through the operation state.

## Retention matrix

| Data                                                        | Retention                                                                |
| ----------------------------------------------------------- | ------------------------------------------------------------------------ |
| Rejected/truncated upload bytes                             | Immediate deletion                                                       |
| Accepted temporary audio                                    | Until explicit deletion or no later than 24 hours after terminal outcome |
| Completed transcript content                                | Until retrieval/deletion or no later than 24 hours after completion      |
| Idempotency tombstone                                       | Seven days after terminal outcome                                        |
| Opaque request/provider correlation, timings, usage outcome | Operational audit policy; no content                                     |
