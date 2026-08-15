# Feature Specification: iOS Foreground Recorder Adapter

**Feature Branch**: `004`

**Created**: 2026-08-15

**Status**: Draft

**Input**: GitHub Issue #4: "Implement the iOS foreground recorder adapter"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Record and finalize a voice memo on iPhone (Priority: P1)

An iPhone user grants microphone access, starts a foreground recording, may
pause and resume it, and stops it to obtain one finalized recording that can be
used by the later transcription journey.

**Why this priority**: A valid, finalized recording is the indispensable native
input for every downstream voice-memo capability.

**Independent Test**: On a physical iPhone running iOS 15 or later, grant
microphone access, record a known phrase, pause and resume once, then stop. The
result must be one playable recording with the expected media type, positive
duration, nonzero byte length, and no active audio session.

**Acceptance Scenarios**:

1. **Given** microphone authorization has not been determined and the app is in
   the foreground, **When** the user starts recording and grants access,
   **Then** exactly one recording session begins and its active state is visible
   to the application.
2. **Given** a recording is active, **When** the user pauses and later resumes,
   **Then** the same session continues without counting the paused interval in
   the captured duration.
3. **Given** a recording is active or paused, **When** the user stops it,
   **Then** exactly one finalized source-audio descriptor is returned and the
   recording and audio session are inactive.

---

### User Story 2 - Handle denial and audio disruptions predictably (Priority: P2)

An iPhone user who denies microphone access or whose recording is interrupted
receives a deterministic outcome that the application can explain and recover
from without hidden capture or silent data loss.

**Why this priority**: iOS permission, call, alarm, and input-route behavior is
unavoidable on physical devices and must not leave the recorder in an unknown
state.

**Independent Test**: On a physical iPhone, separately exercise denied
permission, restricted permission where available, an audio-session
interruption, input-route loss, and foreground exit. Each case must produce one
documented reason and leave no continuing background capture.

**Acceptance Scenarios**:

1. **Given** microphone permission is denied or restricted, **When** recording
   is requested, **Then** no file capture begins and the application receives a
   stable permission outcome with an appropriate settings-recovery indication.
2. **Given** recording is active, **When** an audio interruption begins,
   **Then** capture ends and finalizes recoverable audio when possible, never
   auto-resumes, and reports the interruption outcome once.
3. **Given** recording is active, **When** the active input route is lost or
   changes in a way that threatens integrity, **Then** capture ends
   deterministically and reports a sanitized route-change reason.
4. **Given** recording is active, **When** the app leaves the foreground,
   **Then** capture ends and attempts to finalize; it never continues as a
   background or lock-screen recording.

---

### User Story 3 - Cancel safely and leave no abandoned audio (Priority: P3)

An iPhone user cancels an active or paused recording and can trust that its
temporary audio is removed, while any cleanup problem remains visible to the
application for recovery.

**Why this priority**: Voice content is sensitive, and cancellation must have
reliable privacy semantics rather than being only a UI state change.

**Independent Test**: Start and cancel a recording on a physical iPhone, then
verify that no playable artifact or active audio session remains. Simulate a
cleanup failure and verify that a sanitized cleanup-pending result identifies
the session without exposing its path.

**Acceptance Scenarios**:

1. **Given** a recording is active or paused, **When** the user cancels it,
   **Then** capture and the audio session stop and all temporary artifacts for
   that session are removed.
2. **Given** temporary-artifact deletion cannot complete, **When** cancellation
   returns, **Then** the application receives a cleanup-pending or cleanup-failed
   outcome that can be retried without exposing raw paths or audio content.
3. **Given** stop or cancel is requested repeatedly for the same session,
   **When** commands are processed, **Then** the result is idempotent and no
   second file or conflicting terminal event is produced.

### Edge Cases

- Repeated start requests while a session is active do not create another
  recorder or overwrite the active artifact.
- Pause is valid only while recording; resume is valid only while paused;
  repeated pause or resume requests return the current stable state.
- Stop immediately after start either returns a valid short recording or a
  typed unusable-recording failure and never a partially written file.
- An encoder or finalization failure removes unusable artifacts or reports a
  cleanup-pending outcome associated with the session.
- An interruption, route-change event, foreground exit, and user stop arriving
  close together produce one terminal outcome for the session.
- Output metadata is derived from the finalized file and cannot claim success
  for a missing, empty, or unreadable artifact.
- Wired or Bluetooth input removal never causes capture to continue silently
  through an unintended microphone.
- App termination receives only best-effort platform cleanup; recovered files
  are not uploaded or exposed automatically and full relaunch recovery remains
  owned by the canonical journey follow-up work.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The system MUST expose one platform-neutral recorder contract for
  permission inspection/request, start, pause, resume, stop, and cancel, with
  no iOS framework types crossing into shared application code.
- **FR-002**: The system MUST permit only one active recording session
  application-wide and MUST validate every operation against a stable session
  identifier.
- **FR-003**: The system MUST distinguish idle, recording, paused, finalizing,
  finalized, cancelled, and failed recorder states and reject invalid state
  transitions with stable, sanitized reason codes.
- **FR-004**: The system MUST request microphone authorization only from a
  user-initiated recording action and MUST distinguish undetermined, granted,
  denied, and restricted outcomes supported by iOS.
- **FR-005**: The system MUST configure the native audio session only for
  foreground voice capture and MUST deactivate and restore it after every stop,
  cancellation, and failure path.
- **FR-006**: The system MUST start, pause, and resume capture without creating a
  second logical session, and paused time MUST NOT be included in recorded
  duration.
- **FR-007**: A successful stop MUST return exactly one finalized recording in
  a backend-supported `audio/mp4` container with an `.m4a` app-private artifact,
  positive duration, nonzero byte length, and relevant sanitized metadata.
- **FR-008**: The system MUST verify that a finalized artifact exists, is
  readable, and has internally consistent metadata before reporting stop as
  successful.
- **FR-009**: Cancel MUST stop capture and remove all temporary artifacts for the
  target session; incomplete cleanup MUST be reported as retryable cleanup work.
- **FR-010**: Stop and cancel MUST be idempotent per session and concurrent
  terminal triggers MUST produce only one terminal result.
- **FR-011**: Audio-session interruptions MUST stop capture, attempt to finalize
  recoverable audio, report a stable interruption reason, and MUST NOT
  automatically resume.
- **FR-012**: Input route loss or a route change that risks recording integrity
  MUST stop capture and report a stable route-change reason.
- **FR-013**: Leaving the foreground MUST stop and attempt to finalize active
  capture; recording MUST NOT continue in the background or on the lock screen.
- **FR-014**: The recorder boundary MUST expose sanitized events and errors
  required for application reconciliation without raw native errors, audio
  bytes, or absolute file paths in logs or analytics.
- **FR-015**: Automated contract tests MUST cover permission mapping, valid and
  invalid transitions, idempotent terminal actions, interruption, route change,
  metadata validation, and cleanup outcomes without requiring a live
  microphone.

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: This feature targets iOS 15+ on physical iPhone hardware. Native
  Android recording, permissions, and lifecycle behavior remain out of scope,
  but the shared Rust plugin MUST initialize safely on Android and recorder
  commands MUST return `unsupportedPlatform` until a separate Android adapter
  is specified.
- **MLR-002**: The app MUST include a clear microphone usage description and
  grant only the recorder commands and native capabilities required by this
  foreground feature.
- **MLR-003**: The native recorder MUST observe iOS audio-session interruption,
  media-services reset, and input-route change notifications and map them to the
  platform-neutral outcomes defined above.
- **MLR-004**: Permission denial, interruption, cancellation, route change, five
  consecutive cold launches, pause/resume, successful finalization, and
  foreground exit MUST be verified on a physical iPhone.
- **MLR-005**: Simulator validation MAY supplement automated and physical-device
  evidence but MUST NOT satisfy the feature's completion gate by itself.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

- **PDL-001**: Audio MUST be created only in the application's private local
  storage and MUST NOT be transmitted by this feature.
- **PDL-002**: Active capture uses a temporary session artifact; successful stop
  promotes it to finalized source audio for later application use, while cancel
  and unrecoverable failure delete it.
- **PDL-003**: A finalized source recording MUST remain local until a later
  application use case applies the canonical retention or deletion policy. This
  adapter does not automatically delete a successful recording before its
  consumer can upload or persist it.
- **PDL-004**: Raw audio, audio content, absolute paths, native error text, and
  future provider credentials MUST be excluded from default logs and analytics.
- **PDL-005**: Cleanup-pending artifacts MUST remain app-private, associated with
  a sanitized session identity, and visible to application-level recovery.

### Finalized-audio retention exception

- Issue #4 retains a successful source recording in app-private storage because
  it has no upload, memo, or user-choice surface that can safely decide its fate.
- The adapter never transmits the recording and deletes active or unusable
  artifacts on cancel/failure; a repeated cancel retries incomplete cleanup.
- This is a temporary Constitution IV exception, not a permanent retention
  policy. It ends in Issue #6, which MUST present the source-audio retention
  decision, delete unretained audio after the upload/transcription handoff, and
  expose deletion recovery. Issue #7 MUST keep source-audio deletion independent
  from memo-text retention.
- Until Issue #6 satisfies those conditions, this adapter may be validated and
  integrated as a dependency but the complete record-to-memo lifecycle MUST NOT
  be declared production-ready.

### Architecture Impact _(mandatory)_

- **AI-001**: Pure recorder states, permission outcomes, descriptors, and errors
  belong to the Rust domain boundary; recorder operations belong to an outbound
  port that has no dependency on Tauri or iOS frameworks.
- **AI-002**: Application use cases coordinate recorder operations through the
  port; a thin inbound mobile command boundary validates input and delegates.
- **AI-003**: The Swift implementation and app-private filesystem/audio-session
  behavior belong to an iOS infrastructure adapter behind the same contract.
- **AI-004**: Any React integration in this feature is limited to a deliberate
  shared recorder client contract; recording-session UI ownership remains
  deferred to the journey integration feature and no remote state is added.

### Key Entities

- **Recording Session**: One foreground capture attempt identified by a stable
  opaque ID, with current state, start time, accumulated duration, and terminal
  reason.
- **Permission Outcome**: The normalized microphone authorization state and
  whether settings recovery is appropriate.
- **Finalized Recording**: Verified app-private source audio identified by an
  opaque artifact ID, media type, duration, byte length, integrity metadata,
  and finalization reason.
- **Recorder Event**: A sanitized state or terminal notification associated
  with one session, used to reconcile native lifecycle changes.
- **Cleanup Outcome**: Confirmation that session artifacts were removed or a
  retryable indication that app-private cleanup remains pending.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: On a physical iPhone, 100% of 20 normal start/pause/resume/stop
  trials produce exactly one playable finalized recording with valid metadata.
- **SC-002**: Permission denial or restriction prevents capture in 100% of test
  trials and produces a stable recovery outcome within one second of the user
  action completing.
- **SC-003**: Interruption, input-route loss, foreground exit, and cancellation
  each leave no active capture or active recording audio session in 100% of
  physical-device trials.
- **SC-004**: Cancelled and unrecoverably failed sessions leave no temporary
  audio artifact after successful cleanup in 100% of automated and
  physical-device validation cases.
- **SC-005**: Five consecutive physical-iPhone cold launches followed by one
  successful recording each complete without crash, hung recorder state, or
  duplicate terminal event.
- **SC-006**: All recorder contract tests pass without microphone hardware and
  every contract error exposed outside the adapter uses a documented sanitized
  reason code.
- **SC-007**: On a physical Android device at API 24 or later, the application
  cold-starts without recorder plugin initialization failure and an attempted
  recorder command returns the documented `unsupportedPlatform` result.

## Assumptions

- GitHub Issue #2 and its recorder contract are the canonical cross-platform
  journey dependency; this feature specializes that contract for iOS and adds
  the requested pause/resume semantics without changing downstream
  transcription behavior.
- The source recording is AAC audio in an `.m4a` container (`audio/mp4`), which
  is accepted by the planned application backend.
- Recording is foreground-only; background modes, lock-screen capture,
  realtime transcription, VAD, silence removal, upload, and memo persistence
  are excluded.
- Automatic recovery after process termination is limited to safe cleanup and
  app-private artifacts; user-facing relaunch recovery is implemented by the
  later journey integration work.
- Physical-device verification requires a developer-signed iOS build and manual
  triggering of system interruption and route-change scenarios.

## Dependencies

- GitHub Issue #2 and `specs/002-record-transcribe-journey/` define the canonical
  foreground recording-to-memo behavior and recorder semantics.
- `docs/tauri-mobile-voice-memo.md` defines the project-wide mobile recorder
  boundary and security posture.
- `docs/handy-mobile-code-reuse.md` is behavioral inspiration only; Handy VAD
  and silence removal are explicitly excluded.

## Out of Scope

- Native Android recorder implementation or Android permission/lifecycle changes
- Background or lock-screen recording and background audio entitlement
- Realtime/streaming transcription, backend upload, or OpenAI integration
- VAD, silence removal, normalization, or other derived-audio processing
- Recording-session UI, transcript editing, memo persistence, and desktop release
