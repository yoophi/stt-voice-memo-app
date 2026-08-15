# Contract: iOS Foreground Recorder Plugin

## Boundary

Plugin identifier: `recorder`

The Rust plugin surface implements the platform-neutral `RecorderPort`. The
Swift plugin owns iOS frameworks and native file creation. The TypeScript client
invokes only the Rust/Tauri plugin commands and sees no Apple framework type or
absolute file path.

All objects use camelCase over IPC. Success objects and errors are JSON-safe.

## Commands

| Command              | Input                                | Success                                     | Stable failures                                                                                                                                                              |
| -------------------- | ------------------------------------ | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `permission_status`  | none                                 | `PermissionOutcome`                         | `unsupportedPlatform`                                                                                                                                                        |
| `request_permission` | none                                 | `PermissionOutcome`                         | `permissionRequestUnavailable`, `unsupportedPlatform`                                                                                                                        |
| `recorder_status`    | optional `sessionId`                 | current `RecordingSession` or idle snapshot | `staleSession`, `unsupportedPlatform`                                                                                                                                        |
| `start`              | `{ sessionId }`                      | recording session snapshot                  | `invalidSessionId`, `activeSessionExists`, `permissionDenied`, `permissionRestricted`, `storageUnavailable`, `audioSessionFailure`, `recorderFailure`, `unsupportedPlatform` |
| `pause`              | `{ sessionId }`                      | paused session snapshot                     | `staleSession`, `invalidTransition`, `recorderFailure`, `unsupportedPlatform`                                                                                                |
| `resume`             | `{ sessionId }`                      | recording session snapshot                  | `staleSession`, `invalidTransition`, `audioSessionFailure`, `recorderFailure`, `unsupportedPlatform`                                                                         |
| `stop`               | `{ sessionId, reason?: "userStop" }` | `FinalizedRecording`                        | `staleSession`, `invalidTransition`, `finalizationFailure`, `invalidArtifact`, `cleanupFailure`, `unsupportedPlatform`                                                       |
| `cancel`             | `{ sessionId }`                      | `CleanupOutcome`                            | `staleSession`, `invalidTransition`, `cleanupFailure`, `unsupportedPlatform`                                                                                                 |

`start` is the only command that may request permission implicitly, and only
because it represents a user-initiated Record action. The explicit permission
command exists for the same user action when the UI wants a separate step.

## Idempotency and concurrency

- `pause` on paused and `resume` on recording return the current snapshot.
- The first terminal trigger stores its result keyed by `sessionId`.
- Repeated `stop` returns the same finalized descriptor if stop/finalization won.
- Repeated `cancel` returns `removed` or `notFound` if cancel won.
- Repeated `cancel` after `pending` or `failed` cleanup invokes deletion again;
  success stores the stable cancelled outcome.
- A conflicting terminal command returns the stored terminal outcome as a typed
  conflict and never creates/deletes a second artifact.
- Native interruption, route change, media reset, foreground exit, user stop,
  and cancel are serialized by the same terminal gate.

## Native plugin methods

Rust delegates the same snake-case command names to Swift with normalized input.
Swift may include `fileUri` in its stop result to Rust. The Rust adapter validates
and consumes that field before forming the public stop result. Swift constructs a
separate event recording projection containing the full public descriptor and no
`fileUri`; the TypeScript boundary validates that projection again.

## Event stream

Plugin listener name: `recorderEvent`

Payload is the `RecorderEvent` from `data-model.md`. Events are emitted for:

- recording started;
- paused;
- resumed;
- terminal finalized after interruption, route change, foreground exit, media
  reset, or user stop;
- cancelled;
- terminal failure with cleanup outcome.

Command responses remain authoritative for the command caller. Events exist for
native lifecycle changes and cross-surface reconciliation. No event contains
audio bytes, an absolute path, a native exception description, or transcript.

## Permissions and capability

The plugin build generates allow/deny permissions for every command. The app
capability grants only:

- `recorder:allow-permission-status`
- `recorder:allow-request-permission`
- `recorder:allow-recorder-status`
- `recorder:allow-start`
- `recorder:allow-pause`
- `recorder:allow-resume`
- `recorder:allow-stop`
- `recorder:allow-cancel`

No Tauri filesystem permission is granted. The Swift adapter owns its fixed
app-private directory internally.

## iOS host requirements

- Deployment target: iOS 15 or later.
- `NSMicrophoneUsageDescription` clearly states that audio is recorded only
  when the user starts a voice memo.
- No `UIBackgroundModes` audio entry.
- Audio session category `.record`, mode `.default`, activated only while used.
- Observe interruption, route change, media services reset, and application
  background notifications.

## Desktop and Android behavior

Host desktop calls return `unsupportedPlatform` and perform no filesystem/audio
work. On Android the Rust plugin initializes without registering a native
recorder adapter, preserving application startup; recorder commands return the
same sanitized `unsupportedPlatform` result. A future Android implementation
must conform to the normalized public contract in a separate issue.

## Contract test matrix

| Case                               | Required assertion                                          |
| ---------------------------------- | ----------------------------------------------------------- |
| Permission mapping                 | Every OS state becomes one normalized outcome               |
| Start exclusivity                  | Second session cannot begin while one is active             |
| Pause/resume                       | Same ID, valid state, paused time excluded                  |
| Invalid transitions                | Stable sanitized error; no port side effect                 |
| Stop success                       | One descriptor, valid metadata, terminal state              |
| Repeated stop                      | Same descriptor; no second native stop                      |
| Cancel success                     | Artifact removed; repeated cancel is safe                   |
| Cleanup failure                    | App-private artifact and retryable outcome are represented  |
| Interruption/route/background race | One terminal event and result                               |
| Invalid native artifact            | No public success and cleanup attempted                     |
| Logging                            | No path, audio content, native message, or credential field |
