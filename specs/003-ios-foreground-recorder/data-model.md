# Data Model: iOS Foreground Recorder Adapter

## Design rules

- Domain values contain no Tauri or Apple framework types.
- IDs and reason codes are safe to log; audio paths, bytes, and native error
  descriptions are not.
- One recording session owns at most one temporary artifact and one terminal
  result.
- Native file locators exist only inside the trusted adapter boundary.

## RecordingSessionId

Opaque UUID string generated before native capture begins.

Validation:

- Canonical UUID textual form.
- Required on every state-changing operation.
- Stable across pause/resume and repeated terminal requests.

## RecordingState

| State        | Meaning                                              | Allowed next states                              |
| ------------ | ---------------------------------------------------- | ------------------------------------------------ |
| `idle`       | No active session                                    | `recording`                                      |
| `recording`  | Native capture is writing audio                      | `paused`, `finalizing`, `cancelled`, `failed`    |
| `paused`     | Same recorder/session is open but not capturing      | `recording`, `finalizing`, `cancelled`, `failed` |
| `finalizing` | First terminal trigger is closing/verifying the file | `finalized`, `failed`                            |
| `finalized`  | One verified source recording exists                 | Terminal                                         |
| `cancelled`  | Capture ended and deletion completed                 | Terminal                                         |
| `failed`     | Capture/finalization ended with typed failure        | Terminal                                         |

Invariants:

- The service has zero or one nonterminal session.
- The Swift coordinator is authoritative for live capture and OS-driven terminal
  transitions. The Rust application lifecycle is a portable command-side mirror
  and MUST refresh from native status before returning status or replacing a
  possibly stale active session.
- Repeated pause while paused and resume while recording return the stable
  current snapshot without another native action.
- The first terminal trigger owns finalization. Later stop/cancel/lifecycle
  triggers receive the stored terminal outcome.
- A new session can start only after the previous session is terminal.

## PermissionOutcome

| Field             | Type                                              | Rules                                          |
| ----------------- | ------------------------------------------------- | ---------------------------------------------- |
| `state`           | `undetermined \| granted \| denied \| restricted` | Normalized, never raw numeric OS value         |
| `canRequest`      | boolean                                           | True only when an OS prompt may still be shown |
| `canOpenSettings` | boolean                                           | True for a user-recoverable denial             |

Permission is inspected for each start. Only a user-initiated start or explicit
request operation may show the system prompt.

## RecordingSession

| Field            | Type                      | Rules                                                                          |
| ---------------- | ------------------------- | ------------------------------------------------------------------------------ |
| `sessionId`      | `RecordingSessionId`      | Stable and unique                                                              |
| `state`          | `RecordingState`          | Follows transition table                                                       |
| `startedAtMs`    | integer                   | Monotonic/session-relative for duration; wall-clock value is optional metadata |
| `durationMs`     | nonnegative integer       | Excludes paused intervals                                                      |
| `terminalReason` | optional `TerminalReason` | Set once                                                                       |

## NativeRecordingLocator

Adapter-private value returned from Swift to Rust.

| Field          | Type             | Rules                                                      |
| -------------- | ---------------- | ---------------------------------------------------------- |
| `fileUri`      | string           | `file:` URI under the recorder-owned app-private directory |
| `container`    | `m4a`            | Fixed for this feature                                     |
| `durationMs`   | positive integer | Measured after recorder stop                               |
| `sampleRateHz` | positive integer | Read from final media metadata                             |
| `channelCount` | positive integer | One for configured output                                  |

This entity is never serialized to React events, logs, or analytics.

## FinalizedRecording

| Field                | Type                                                        | Rules                                     |
| -------------------- | ----------------------------------------------------------- | ----------------------------------------- |
| `artifactId`         | opaque UUID                                                 | Public identity, not a path               |
| `sessionId`          | `RecordingSessionId`                                        | Owning session                            |
| `mimeType`           | `audio/mp4`                                                 | Fixed and verified                        |
| `fileExtension`      | `m4a`                                                       | Fixed and verified                        |
| `durationMs`         | positive integer                                            | Excludes paused intervals                 |
| `byteLength`         | positive integer                                            | Read after close                          |
| `sampleRateHz`       | positive integer                                            | Sanitized metadata                        |
| `channelCount`       | positive integer                                            | Sanitized metadata                        |
| `sha256`             | lowercase hex string                                        | Integrity/correlation, content not logged |
| `finalizationReason` | `userStop \| interruption \| routeChange \| foregroundExit` | Why capture ended                         |

Successful creation requires a readable nonempty file under the owned directory
and consistent container/metadata. Artifact identity is persisted only as
needed for the later recording-file access port.

## TerminalReason

Sanitized reason enum:

- `userStop`
- `userCancel`
- `interruption`
- `routeChange`
- `foregroundExit`
- `mediaServicesReset`
- `permissionDenied`
- `permissionRestricted`
- `storageUnavailable`
- `audioSessionFailure`
- `recorderFailure`
- `finalizationFailure`
- `cleanupFailure`
- `unsupportedPlatform`

Native exception names, localized messages, file paths, and OS numeric values do
not cross the adapter boundary.

## RecorderError

| Field       | Type                      | Rules                                               |
| ----------- | ------------------------- | --------------------------------------------------- |
| `code`      | stable string enum        | Derived from `TerminalReason` or validation failure |
| `sessionId` | optional ID               | Present when a session exists                       |
| `retryable` | boolean                   | True only when repeating a safe action may recover  |
| `cleanup`   | optional `CleanupOutcome` | Present if audio may remain                         |

Additional validation codes include `invalidSessionId`, `activeSessionExists`,
`invalidTransition`, `staleSession`, and `invalidArtifact`.

## CleanupOutcome

| Value      | Meaning                                                           |
| ---------- | ----------------------------------------------------------------- |
| `removed`  | No temporary artifact remains                                     |
| `notFound` | Idempotent success; no artifact existed                           |
| `pending`  | Artifact remains app-private and cleanup may be retried           |
| `failed`   | Retry did not remove the artifact; application attention required |

## RecorderEvent

| Field       | Type                          | Rules                             |
| ----------- | ----------------------------- | --------------------------------- |
| `eventId`   | UUID                          | Deduplication identity            |
| `sessionId` | `RecordingSessionId`          | Owning session                    |
| `sequence`  | positive integer              | Strictly increases per session    |
| `state`     | `RecordingState`              | State after event                 |
| `reason`    | optional `TerminalReason`     | Sanitized only                    |
| `recording` | optional `FinalizedRecording` | Only for finalized terminal event |
| `cleanup`   | optional `CleanupOutcome`     | Only for cancel/failure cleanup   |

Events are advisory for Issue #4. Issue #6 will reconcile them with durable
journey state; consumers must deduplicate by `eventId` and prefer higher
sequence values for a session.
