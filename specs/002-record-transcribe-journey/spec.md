# Feature Specification: Record and Transcribe Memo Journey

**Feature Branch**: `002-record-transcribe-journey`

**Created**: 2026-08-15

**Status**: Draft

**Input**: GitHub Issue #2: "Specify the record-and-transcribe memo journey"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Turn a recording into an editable memo (Priority: P1)

A mobile user records a short voice memo while the app is in the foreground,
stops recording, waits for transcription, reviews and edits the returned text,
and saves it as a memo.

**Why this priority**: This is the smallest complete product journey and the
reason the application exists.

**Independent Test**: On each supported mobile platform, grant microphone
access, record a known phrase, stop, receive text, edit one word, and save. The
saved memo must contain the edited text exactly once and remain available after
relaunch.

**Acceptance Scenarios**:

1. **Given** microphone permission is available and the app is foregrounded,
   **When** the user starts and stops a valid recording, **Then** the app shows
   recording, finalizing, upload, transcription, and editable-draft progress in
   an understandable order.
2. **Given** an editable transcript has been returned, **When** the user changes
   its text and saves, **Then** exactly one memo containing the edited text is
   persisted and shown as saved.
3. **Given** the user has not opted to retain source audio, **When** the memo is
   saved successfully, **Then** the local source audio is deleted and the user
   can see that no recording was retained.

---

### User Story 2 - Recover without losing a recording (Priority: P2)

A mobile user receives a permission, interruption, connectivity, or remote
processing failure and is told what happened, what data remains, and whether
they can retry, cancel, or continue editing without accidentally duplicating
work.

**Why this priority**: Recordings are difficult to recreate. Predictable
recovery is essential to user trust and is a release gate for mobile audio.

**Independent Test**: Complete the journey separately with permission denied,
an audio interruption, an offline stop, a timeout, and a repeated retry. Each
case must offer only valid recovery actions, preserve recoverable audio, and
produce no duplicate transcription request or memo.

**Acceptance Scenarios**:

1. **Given** microphone permission is denied or restricted, **When** the user
   tries to record, **Then** no recording begins and the app explains how to
   recover without repeatedly prompting.
2. **Given** a recording is interrupted by the operating system, **When** the
   interruption occurs, **Then** recording stops safely, recoverable audio is
   finalized when possible, and the user chooses whether to continue with it or
   discard it.
3. **Given** finalized audio exists while the device is offline, **When** the
   user requests transcription, **Then** the request is queued locally, the
   audio remains available, and the user can retry or cancel after connectivity
   returns.
4. **Given** a transcription request has an uncertain outcome, **When** the user
   retries, **Then** the retry represents the same operation and cannot create a
   second billable transcription or a duplicate memo.
5. **Given** remote processing fails after upload, **When** the app reports the
   failure, **Then** the source audio remains local, remote temporary data is
   scheduled for deletion, and retry and delete actions are available.

---

### User Story 3 - Control retained voice data (Priority: P3)

A privacy-conscious user understands where their recording and transcript go,
chooses whether to retain the original recording, and can delete either an
unfinished recording or a saved memo and its retained audio.

**Why this priority**: Voice data can be sensitive. Explicit retention and
deletion behavior is required even though the default journey remains fast.

**Independent Test**: Run one journey with the default deletion policy and one
with source-audio retention enabled, then delete the saved memo. At every step,
the visible state and remaining local or remote data must match the selected
policy.

**Acceptance Scenarios**:

1. **Given** finalized audio is awaiting transcription or save, **When** the
   user cancels and confirms deletion, **Then** the local audio and unsaved text
   are deleted and any queued remote work is cancelled or its result ignored.
2. **Given** the user explicitly chooses to keep the original recording,
   **When** the memo is saved, **Then** the recording remains attached to that
   memo until the user deletes it.
3. **Given** a memo has retained audio, **When** the user deletes the memo,
   **Then** both transcript and retained audio are deleted from app-controlled
   storage and the result is visible to the user.

### Edge Cases

- A recording shorter than one second or containing no usable speech is not
  uploaded automatically; the user may discard it or deliberately retry.
- Repeated taps on start, stop, transcribe, retry, or save result in one active
  operation for the current recording session.
- Navigating away while recording requires confirmation; backgrounding ends the
  foreground-only recording and attempts to finalize recoverable audio.
- If the app terminates during recording, it recovers a finalized recording on
  next launch when the platform produced one; otherwise it reports that no
  usable recording survived. It never silently uploads recovered audio.
- If the app terminates during upload or transcription, it restores the pending
  operation from durable metadata and queries or retries the same operation
  identifier after relaunch.
- A slow network exposes ongoing progress and a cancel action without declaring
  failure solely because a fixed UI timer elapsed.
- A response without a usable final transcript is treated as a recoverable
  transcription failure; the source audio remains available and no empty memo
  is auto-saved.
- Insufficient local storage prevents recording from starting or finalizing and
  explains how to free space without claiming that audio was saved.
- An incoming call, audio route change, alarm, or another app taking audio focus
  follows the platform interruption path and never continues recording
  invisibly.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The system MUST support one foreground recording session at a time
  and MUST make idle, permission, recording, finalizing, ready-to-transcribe,
  queued, uploading, transcribing, editable-draft, saving, saved, cancelled, and
  recoverable-failure states distinguishable to the user.
- **FR-002**: The system MUST prevent invalid or duplicate actions during every
  state transition, including repeated start, stop, transcribe, retry, and save
  actions.
- **FR-003**: The system MUST finalize a stopped recording into a locally
  identifiable source-audio item before any upload begins.
- **FR-004**: The system MUST require an explicit user action to send finalized
  audio for transcription and MUST identify that operation consistently across
  retries and app relaunches.
- **FR-005**: The system MUST present a returned final transcript as an editable
  draft and MUST NOT save an empty or unusable final transcript automatically.
- **FR-006**: The system MUST persist exactly one memo from a successful save
  action and MUST preserve the user's edited text.
- **FR-007**: The system MUST retain finalized local source audio while upload,
  transcription, or save is pending or recoverably failed.
- **FR-008**: The system MUST delete local source audio after successful memo
  save by default, unless the user explicitly chooses to retain it.
- **FR-009**: The system MUST allow the user to cancel an unfinished journey and
  delete its local audio and draft after a clear confirmation.
- **FR-010**: The system MUST restore recoverable finalized recordings and
  pending operation metadata after app relaunch without silently uploading them.
- **FR-011**: The system MUST distinguish retryable connectivity or service
  failures from non-retryable content or authorization failures and offer only
  applicable recovery actions.
- **FR-012**: The system MUST use one stable recording identifier and one stable
  transcription operation identifier so duplicate client submissions resolve to
  the same logical transcription and memo journey.
- **FR-013**: The system MUST never place OpenAI credentials or other backend
  secrets in the Tauri client, mobile bundle, client persistence, diagnostics,
  or analytics.
- **FR-014**: The system MUST exclude raw audio, transcript text, credentials,
  and signed upload locations from logs and analytics by default.
- **FR-015**: The system MUST expose whether source audio is temporary, retained,
  pending deletion, or deleted wherever the user makes a retention or deletion
  choice.
- **FR-016**: The first release MUST NOT record in the background, provide
  realtime/streaming transcription, or expand the recording journey to desktop.

### State and Recovery Contract

| Current condition                | User or system event              | Required result                                                           | Recoverable data                               |
| -------------------------------- | --------------------------------- | ------------------------------------------------------------------------- | ---------------------------------------------- |
| Idle / permission unknown        | Start requested                   | Request permission once and wait                                          | None                                           |
| Permission denied/restricted     | Start requested                   | Do not record; explain settings/recovery                                  | None                                           |
| Permission granted               | Start requested                   | Create one recording session                                              | Session metadata                               |
| Recording                        | Stop, background, or interruption | Stop capture and finalize when possible                                   | Finalized source audio or explicit loss result |
| Recording                        | User cancels                      | Stop capture and confirm deletion                                         | Audio until deletion confirmed                 |
| Ready to transcribe              | User submits while online         | Start one identified upload/transcription operation                       | Source audio + operation metadata              |
| Ready to transcribe              | User submits while offline        | Queue the same operation locally                                          | Source audio + operation metadata              |
| Uploading/transcribing           | Cancel requested                  | Stop client work where possible; ignore late results; offer deletion      | Source audio until user decision               |
| Uploading/transcribing           | Retryable or uncertain failure    | Preserve identifiers and offer same-operation retry                       | Source audio + operation metadata              |
| Uploading/transcribing           | Non-retryable failure             | Explain failure; offer delete or a new deliberate attempt when applicable | Source audio until user decision               |
| Transcribing                     | Usable final transcript received  | Present one editable draft                                                | Draft + source audio                           |
| Editable draft                   | Save requested                    | Persist one memo, then apply audio retention choice                       | Memo; audio only when retained                 |
| Any recoverable unfinished state | App relaunch                      | Restore state without automatic upload                                    | Durable local artifacts                        |

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: iOS 15+ MUST use the system microphone authorization status and
  show an app-settings recovery path after denial; Android API 24+ MUST use the
  runtime microphone permission and distinguish denial from a permanently denied
  state where the platform supports that distinction.
- **MLR-002**: On both platforms, moving the app out of the foreground ends the
  active recording and attempts to finalize it. Recording MUST NOT continue in
  the background for this feature.
- **MLR-003**: iOS audio-session interruptions and route changes and Android
  audio-focus or route interruptions MUST lead to the same user-visible stopped,
  recoverable, or lost outcomes even when native platform mechanics differ.
- **MLR-004**: Offline, slow-network, cancellation, retry, interruption, and app
  termination behavior MUST be validated separately on a physical iOS device and
  a physical Android device; simulator/emulator evidence is supplementary.
- **MLR-005**: Primary controls, progress, errors, and retention choices MUST be
  usable with touch and exposed to the platform accessibility service with clear
  labels and state announcements.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

| Artifact                   | Created at                                       | Transmitted to                                                       | Retained until                                                           | Deleted by                            |
| -------------------------- | ------------------------------------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------- |
| Recording session metadata | Mobile client at start                           | App backend only when submission begins                              | Journey completes or is cancelled                                        | Journey cleanup                       |
| Original source audio      | Mobile device during recording/finalization      | Application-controlled backend after explicit submission             | Successful save, confirmed cancellation, or user-selected retention      | Client cleanup or later memo deletion |
| Backend upload copy        | Application backend                              | Approved transcription provider                                      | Processing reaches a terminal result; deletion completes within 24 hours | Backend cleanup job                   |
| Provider processing copy   | Approved transcription provider                  | No additional recipient by this feature                              | Provider/API contract selected by the backend                            | Provider under the agreed data policy |
| Editable transcript draft  | Mobile client after successful processing        | No destination until memo save/sync behavior is separately specified | Save or confirmed cancellation                                           | Draft cleanup                         |
| Saved transcript memo      | Mobile app-controlled storage                    | Only a future sync service defined by a separate feature             | User deletes the memo                                                    | Memo deletion                         |
| Optional retained audio    | Mobile app-controlled storage attached to a memo | No automatic retransmission                                          | User deletes retained audio or its memo                                  | User-initiated deletion               |

- **PDL-001**: Submission UI MUST tell the user that audio leaves the device for
  backend-mediated transcription before the first submission.
- **PDL-002**: Cancellation or deletion MUST invalidate queued work and ignore a
  late remote result so deleted content is not recreated in the client.
- **PDL-003**: Backend design MUST document and verify terminal cleanup within 24
  hours; provider retention terms MUST be recorded before production enablement.
- **PDL-004**: Any future derived audio is a separate temporary artifact and MUST
  NOT replace the original or inherit its retention choice implicitly.

### Architecture Impact _(mandatory)_

- Rust domain/application work will define journey entities, valid state
  transitions, idempotent operation identity, and use cases behind recorder,
  transcription, memo-repository, and cleanup ports. Platform, filesystem,
  network, backend, and Tauri behavior remain adapters.
- React work will place the mobile page composition in `pages`, journey actions
  in `features`, memo/recording representations in `entities`, and platform/API
  clients plus reusable UI in `shared`, preserving downward dependency flow.
- TanStack Query will own upload/transcription/save remote operations and their
  retry state. Zustand may own only the active local recording/UI session. Saved
  memo data and remote operation state MUST NOT be duplicated into Zustand.
- Native iOS/Android recording, backend API implementation, Rust application
  implementation, end-to-end integration, and final memo UI remain the scopes of
  Issues #3 through #7. This feature establishes their shared behavior contract
  and does not add microphone permissions or production controls by itself.

### Key Entities _(include if feature involves data)_

- **Recording Session**: One foreground capture attempt, identified independently
  of its resulting file and carrying permission, lifecycle, timing, and outcome
  state.
- **Source Audio**: The finalized original recording with a stable identifier,
  format, duration, local location, integrity metadata, and retention state.
- **Transcription Operation**: One retry-safe request associated with one source
  audio item, including its stable operation identity and processing outcome.
- **Transcript Draft**: Editable text returned from a completed transcription
  operation but not yet committed as a memo.
- **Memo**: The locally persisted edited transcript and metadata, optionally
  linked to explicitly retained source audio.
- **Retention Choice**: The user's explicit decision to delete source audio after
  save or keep it with the memo.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: At least 90% of first-time test participants can record, submit,
  edit, and save a two-minute-or-shorter memo without assistance on each mobile
  platform.
- **SC-002**: Every accepted user action displays its new state or progress
  acknowledgment within one second, excluding completion time for recording,
  network transfer, and remote processing.
- **SC-003**: In 100 repeated-tap and retry test runs per platform, each source
  recording produces at most one logical transcription operation and one saved
  memo.
- **SC-004**: In all offline, interruption, slow-network, cancellation, and app
  relaunch acceptance scenarios, the app either recovers the finalized source
  audio or explicitly reports that it was not recoverable; it never silently
  loses or uploads a known recoverable recording.
- **SC-005**: The primary success journey and at least one permission,
  interruption, offline-retry, cancellation, and termination recovery journey
  pass on one physical iOS 15+ device and one physical Android API 24+ device.
- **SC-006**: Security inspection finds zero OpenAI/backend credentials and zero
  raw audio or transcript content in client bundles, persisted diagnostics,
  default logs, and analytics events.
- **SC-007**: In all retention-policy tests, observable local and backend data
  matches the user's choice, and backend temporary upload deletion completes
  within 24 hours of a terminal outcome.

## Assumptions

- The first release targets short, foreground voice memos; two minutes is the
  usability benchmark, while hard size and duration limits will be selected with
  the backend contract in Issue #3 and shown before recording.
- Network access is required for transcription, but recording and retaining a
  retryable local item work offline.
- Users explicitly initiate transcription; recovered recordings are never sent
  automatically after relaunch.
- Original audio deletion after successful save is the privacy-first default.
- Authentication, synchronization, background recording, realtime
  transcription, desktop recording, and optional audio preprocessing require
  separate specifications or the already planned follow-up issues.
- No clarification questions are required for this specification because Issue
  #2, the project constitution, and the documented privacy-first defaults resolve
  all material product-scope decisions.
