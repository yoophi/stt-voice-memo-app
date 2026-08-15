# Data Model: Record and Transcribe Memo Journey

## Scope and ownership

This is the canonical behavioral model for Issues #3 through #7. It describes
identities, invariants, and state transitions without selecting a database,
serialization format, native API, or HTTP shape.

Rust domain/application code will eventually own transition rules. Durable local
repositories will own relaunch recovery. React may project this state for display
but is not a second source of truth.

## Stable identifiers

| Identifier                 | Lifetime                                      | Rule                                                                 |
| -------------------------- | --------------------------------------------- | -------------------------------------------------------------------- |
| `RecordingSessionId`       | From Record tap through cleanup               | New for every deliberate capture attempt; never reused               |
| `SourceAudioId`            | From successful finalization through deletion | Exactly one original source artifact per finalized session           |
| `TranscriptionOperationId` | From first submit through terminal cleanup    | Stable across offline queueing, timeout, relaunch, and retry         |
| `MemoId`                   | From first successful save through deletion   | Exactly one per committed draft; repeated save returns the same memo |

Identifiers are opaque, non-secret, and safe to log. They do not contain a file
path, transcript text, account email, or device identifier.

## RecordingSession

Represents one foreground capture attempt.

| Field               | Meaning / invariant                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------------------- |
| `id`                | Stable `RecordingSessionId`                                                                               |
| `permissionOutcome` | `unknown`, `granted`, `denied`, or `restricted`; recording requires `granted`                             |
| `captureState`      | Current capture state from the state machine below                                                        |
| `startedAt`         | Present only after capture starts                                                                         |
| `stoppedAt`         | Present after capture ends                                                                                |
| `stopReason`        | User stop, cancellation, background, interruption, route change, encoder failure, or termination recovery |
| `sourceAudioId`     | Present only after usable audio finalizes                                                                 |

### Capture states

```text
idle
  -> requesting_permission
  -> permission_denied
  -> recording
  -> finalizing
  -> finalized
  -> cancelled
  -> unrecoverable
```

### Invariants

- At most one session is `recording` or `finalizing`.
- Repeated start or stop events are no-ops with an observable existing-state
  result; they never create a second session or source file.
- Background and interruption outcomes never transition back to `recording`
  automatically.
- `finalized` requires one readable `SourceAudio`; `unrecoverable` requires an
  explicit reason and no claim that audio was saved.

## SourceAudio

Represents the untouched, finalized recording used for transcription and
optional later retention.

| Field                | Meaning / invariant                                                          |
| -------------------- | ---------------------------------------------------------------------------- |
| `id`                 | Stable `SourceAudioId`                                                       |
| `recordingSessionId` | Exactly one owning recording session                                         |
| `mediaType`          | Verified media type; initial mobile preference is `audio/mp4`/m4a            |
| `byteLength`         | Positive and within limits selected by the backend contract                  |
| `durationMs`         | Positive usable duration                                                     |
| `integrity`          | Metadata sufficient to detect a changed/corrupt retry input                  |
| `localState`         | `temporary`, `retained`, `pending_deletion`, or `deleted`                    |
| `remoteState`        | `never_uploaded`, `uploading`, `temporary`, `pending_deletion`, or `deleted` |

### Invariants

- Derived/processed audio, if introduced later, receives a different identity and
  cannot replace this original.
- Local audio remains available through a retryable transcription or save
  failure.
- Successful memo save moves local audio to `retained` only after explicit user
  opt-in; otherwise it moves through `pending_deletion` to `deleted`.
- Cancel/delete invalidates queued remote work before local deletion completes.

## TranscriptionOperation

Represents one logical backend-mediated transcription, not one HTTP or provider
attempt.

| Field             | Meaning / invariant                                                        |
| ----------------- | -------------------------------------------------------------------------- |
| `id`              | Stable `TranscriptionOperationId`; used as the app-backend idempotency key |
| `sourceAudioId`   | One immutable source audio identity                                        |
| `state`           | Current operation state                                                    |
| `attemptCount`    | Diagnostic count; does not change logical identity                         |
| `failureKind`     | Retryable, non-retryable, uncertain, cancelled, or none                    |
| `finalTranscript` | Present only in `completed` and non-blank                                  |

### Operation states

```text
not_requested
  -> queued_offline
  -> uploading
  -> transcribing
  -> completed
  -> retryable_failure
  -> terminal_failure
  -> cancelled
```

From `queued_offline` or `retryable_failure`, retry returns to `uploading` with
the same ID. An uncertain timeout first resolves backend status for the same ID.
Late completion after `cancelled` is ignored and cannot recreate client content.

### Invariants

- One source audio item has at most one active logical operation.
- Partial/streamed text does not populate `finalTranscript`.
- A completed operation can create at most one `TranscriptDraft`.
- Provider model name, API key, signed locations, and provider payloads are not
  client-domain fields.

## TranscriptDraft

Represents the user's editable copy of a completed final transcript.

| Field                      | Meaning / invariant                                         |
| -------------------------- | ----------------------------------------------------------- |
| `transcriptionOperationId` | Exactly one completed source operation                      |
| `originalText`             | Non-blank provider final text                               |
| `editedText`               | Current component-local edit; non-blank before save         |
| `state`                    | `editing`, `saving`, `saved`, or `discarded`                |
| `retentionChoice`          | `delete_after_save` by default or explicit `keep_with_memo` |

The editing buffer is component-local until save. Remote operation state remains
in TanStack Query; it is not copied into Zustand.

## Memo

Represents the committed local product data.

| Field                            | Meaning / invariant                                   |
| -------------------------------- | ----------------------------------------------------- |
| `id`                             | Stable `MemoId`                                       |
| `text`                           | The user's final edited, non-blank transcript         |
| `createdAt` / `updatedAt`        | Local memo chronology                                 |
| `sourceTranscriptionOperationId` | Traceability without transcript/audio content in logs |
| `retainedSourceAudioId`          | Present only for explicit `keep_with_memo`            |

Repeated completion of the same save operation resolves to the same `MemoId`.
Deleting the memo deletes its text and any attached retained source audio.

## Aggregate transition summary

```text
RecordingSession.finalized
  -> SourceAudio.temporary
  -> TranscriptionOperation.queued_offline | uploading
  -> TranscriptionOperation.completed
  -> TranscriptDraft.editing
  -> Memo(saved exactly once)
  -> SourceAudio.deleted | SourceAudio.retained
```

Cancellation is terminal for the current journey. Cleanup can continue after the
user-visible terminal state, but a late remote result cannot leave that state.

## State ownership projection

| State                                                                     | Future owner          | React access                      |
| ------------------------------------------------------------------------- | --------------------- | --------------------------------- |
| Live capture timer, current route, immediate interruption/confirmation UI | Zustand feature store | Direct feature subscription       |
| Durable journey/source metadata and recovery                              | Rust repository       | TanStack Query through entity API |
| Upload/transcription status and retry/cancel                              | Rust/backend          | TanStack Query mutation/query     |
| Transcript edit buffer and dialog visibility                              | React component       | Local component state             |
| Saved memo and retained-audio metadata                                    | Rust memo repository  | TanStack Query                    |
