# Research: iOS Foreground Recorder Adapter

## Scope

This research resolves implementation decisions for GitHub Issue #4. It covers
the Tauri mobile plugin boundary, iOS 15+ foreground recording, permission,
audio-session lifecycle, interruption/route handling, metadata, cleanup, and
verification. Android, upload, transcription, UI workflow, VAD, and background
audio are excluded.

## Decision 1: Use an in-repository Tauri 2 mobile plugin

**Decision**: Create `src-tauri/plugins/recorder` with a Rust plugin facade, a
Swift Package Manager iOS implementation, generated per-command permissions,
and a small TypeScript guest API. Register it through
`tauri_plugin_recorder::init()`.

**Rationale**: Tauri 2 defines native iOS plugins as Swift `Plugin` subclasses
whose `@objc` invoke methods are registered through the Rust plugin binding.
This is the supported boundary for native Swift while keeping application code
free of Apple APIs.

**Alternatives considered**:

- Direct Swift/C FFI from the application crate was rejected because it would
  bypass Tauri command permissions and combine app setup with native details.
- WebView `MediaRecorder` was rejected because it cannot be the only mobile
  recorder and does not provide the required iOS lifecycle contract.
- A separately published plugin was rejected because there is one application
  consumer and no independent versioning requirement.

**Primary source**: [Tauri Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/)

## Decision 2: Keep recorder contracts in a pure Rust crate

**Decision**: Create `src-tauri/crates/recorder-core` for normalized domain
types, the state machine, `RecorderPort`, and application coordination. It has
Serde/thiserror only when serialization or typed errors require them and has no
Tauri, filesystem, network, or OS dependency. The plugin implements the port.

**Rationale**: A separate crate makes the constitution boundary mechanically
testable. It also allows fake-port tests to cover permission, transition,
idempotency, and failure rules without a microphone.

**Alternatives considered**: Modules inside the application or plugin crate
were rejected because their package dependency graph would still include Tauri
and make the purity rule less enforceable.

## Decision 3: Use AVAudioRecorder with AAC in an M4A container

**Decision**: Use `AVAudioRecorder` writing MPEG-4 AAC, mono, 44.1 kHz, at a
speech-appropriate encoder quality to an app-private `.m4a` file. Use its
`pause`, `record` (resume), `stop`, `currentTime`, and `deleteRecording`
semantics.

**Rationale**: The required feature is file-oriented foreground voice capture,
including native pause/resume, not live PCM processing. `AVAudioRecorder`
directly provides those operations and closes the file on stop. M4A is the
canonical mobile source format from Issue #2 and avoids a transcoding step.

**Alternatives considered**:

- `AVAudioEngine` was rejected because advanced PCM processing is out of scope.
- WAV was rejected because it is much larger and would add no value to the
  later backend contract.
- VAD/silence removal was rejected because it would alter the source timeline
  and is explicitly out of scope.

**Primary source**: [Apple AVAudioRecorder](https://developer.apple.com/documentation/avfaudio/avaudiorecorder)

## Decision 4: Use the iOS 15-compatible permission API with an availability path

**Decision**: For iOS 17+, inspect/request permission through
`AVAudioApplication`; for iOS 15–16, use `AVAudioSession.recordPermission` and
`requestRecordPermission`. Normalize `undetermined`, `granted`, and `denied`;
reserve `restricted` for an OS outcome if exposed by the active API. Prompt only
from the user-initiated `start` flow. Add `NSMicrophoneUsageDescription`.

**Rationale**: Apple's current API replaces the older audio-session property,
but the deployment target requires an availability branch. Apple requires a
microphone usage string and may terminate an app that accesses the microphone
without one.

**Alternatives considered**: Prompting at app startup was rejected because the
foundation app must not request sensitive access without clear user intent.

**Primary sources**:

- [Apple requestRecordPermission](https://developer.apple.com/documentation/avfaudio/avaudiosession/requestrecordpermission%28_%3A%29)
- [Apple AVAudioApplication recordPermission](https://developer.apple.com/documentation/avfaudio/avaudioapplication/recordpermission-swift.property)

## Decision 5: Activate a record-only audio session just in time

**Decision**: Immediately before capture, configure the shared audio session as
`.record` with `.default` mode and no background/mixing options, then activate
it. On every terminal path, stop the recorder and deactivate with
`.notifyOthersOnDeactivation`. Do not add `UIBackgroundModes` audio.

**Rationale**: The record category matches an input-only voice memo and silences
unexpected app playback while active. Deferring activation avoids interrupting
other audio before the user starts. No background entitlement is necessary or
permitted for this issue.

**Alternatives considered**:

- `.playAndRecord` was rejected because simultaneous playback is not required.
- Keeping the session active between recordings was rejected because it would
  unnecessarily own/interfere with system audio.

**Primary sources**:

- [Apple AVAudioSession](https://developer.apple.com/documentation/avfaudio/avaudiosession)
- [Apple record category](https://developer.apple.com/documentation/avfaudio/avaudiosession/category-swift.struct/record)
- [Apple setCategory](https://developer.apple.com/documentation/avfaudio/avaudiosession/setcategory%28_%3Amode%3Aoptions%3A%29)

## Decision 6: Serialize state and emit one terminal event

**Decision**: Isolate Swift recorder state on the main actor. A terminal gate
accepts the first stop, cancel, interruption, route-loss, media-services reset,
or foreground-exit trigger for a session and ignores subsequent competing
triggers. Native events use Tauri plugin listeners and contain only an event ID,
session ID, state, reason, recoverability, and optional sanitized descriptor.

**Rationale**: Apple posts interruption and media reset notifications on the
main thread, while user invokes can race with them. One serialized authority and
terminal gate prevent duplicate files/results. Tauri's native plugin listener
mechanism supports Swift-triggered events without exposing iOS types.

**Alternatives considered**: Auto-resume after an interruption was rejected for
privacy and source-integrity reasons. Raw Tauri global events were rejected in
favor of the plugin-scoped listener surface.

**Primary sources**:

- [Apple interruptionNotification](https://developer.apple.com/documentation/avfaudio/avaudiosession/interruptionnotification)
- [Apple mediaServicesWereResetNotification](https://developer.apple.com/documentation/avfaudio/avaudiosession/mediaserviceswereresetnotification)
- [Tauri Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/)

## Decision 7: Stop on risky route changes and foreground exit

**Decision**: Observe route changes and terminate when the previous input is
removed or the active input identity changes during capture. Observe the
application entering background and finalize with reason `foregroundExit`.
Never resume automatically after either event.

**Rationale**: Continuing through an unintended microphone creates an invisible
integrity/privacy change. Foreground-only behavior requires a terminal boundary
before suspension, and no background audio entitlement is added.

**Alternatives considered**: Continuing across every route change was rejected
because route behavior and input quality can change without user intent.

**Primary source**: [Apple Responding to audio route changes](https://developer.apple.com/documentation/avfaudio/responding-to-audio-route-changes)

## Decision 8: Separate native locator from public descriptor

**Decision**: Swift returns an internal app-private `fileUri` only to trusted
Rust adapter code. Rust validates the locator and maps it to a public descriptor
containing an opaque artifact ID, MIME type, duration, byte length, sample rate,
channel count, and SHA-256 checksum. The TypeScript client receives the public
descriptor, never an absolute path.

**Rationale**: Later upload work requires a file locator through a
recording-file port, but the UI and logs do not. Separating representations
preserves least privilege and prevents accidental path disclosure.

**Alternatives considered**: Returning an absolute path to React was rejected
because it leaks an implementation detail and encourages general filesystem
capabilities.

## Decision 9: Generate and grant individual command permissions

**Decision**: Generate permissions for `permission_status`,
`request_permission`, `recorder_status`, `start`, `pause`, `resume`, `stop`, and
`cancel`. The app capability grants these recorder permissions individually; no
filesystem permission or broad plugin default is added.

**Rationale**: Tauri's build helper generates allow/deny permissions for plugin
commands, and capabilities can enable specific command privileges. This is the
smallest surface needed by Issue #4.

**Alternatives considered**: A capability with general filesystem access or
unrelated default commands was rejected as unnecessary.

**Primary sources**:

- [Tauri Writing Plugin Permissions](https://v2.tauri.app/learn/security/writing-plugin-permissions/)
- [Tauri Permissions](https://v2.tauri.app/security/permissions/)

## Decision 10: Layer automated, build, and physical-device evidence

**Decision**: Contract/state behavior is covered by Rust fake-port tests;
coordinator races and cleanup by Swift tests with injected fakes; integration by
Rust/TypeScript compile tests and an iOS device build; actual permission,
interruption, routes, cold launches, and audio validity by the physical-device
matrix.

**Rationale**: Native audio hardware and OS lifecycle behavior cannot be proven
by host unit tests or the simulator. Separating evidence prevents mocks from
being treated as completion proof.

**Alternatives considered**: Simulator-only acceptance was rejected by the
constitution and GitHub issue.
