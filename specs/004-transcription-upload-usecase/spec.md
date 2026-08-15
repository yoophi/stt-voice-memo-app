# Feature Specification: Transcription Upload Use Case

**Feature Branch**: `005-transcription-upload-usecase`

**Created**: 2026-08-15

**Status**: Draft

**Input**: GitHub Issue #5: "Implement TranscriptionPort and the upload use case"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Submit finalized audio for transcription (Priority: P1)

A mobile user with a finalized voice recording can submit that recording to the
application backend and receive one authoritative final transcript without the
app exposing provider details or credentials.

**Why this priority**: This is the smallest application capability that turns a
usable recording into text and unlocks the later record-to-memo integration.

**Independent Test**: Provide a deterministic finalized-audio fixture and an
authorized backend contract double, submit it through the application boundary,
advance the logical operation through accepted and processing states, and verify
that exactly one non-blank final transcript is returned for the same operation.

**Acceptance Scenarios**:

1. **Given** a readable finalized recording within the backend limits and a
   valid user session, **When** transcription is submitted, **Then** one stable
   logical operation is accepted and remains associated with that source audio.
2. **Given** an accepted operation is queued or processing, **When** its status
   is refreshed, **Then** the current non-terminal state and safe polling
   guidance are returned without partial transcript text.
3. **Given** the backend completes the operation with non-blank final text,
   **When** status is refreshed, **Then** the use case returns one final
   transcript and a repeated refresh returns the same logical result.
4. **Given** source metadata is malformed, the artifact is unreadable, or its
   integrity no longer matches, **When** submission is requested, **Then** the
   request fails before upload with a stable, content-free outcome.

---

### User Story 2 - Recover safely from mobile network failures (Priority: P2)

A mobile user can recover from offline state, slow upload, rate limiting,
temporary service failure, an uncertain timeout, or app restart without creating
a second logical transcription or losing the ability to retry.

**Why this priority**: Mobile connections fail routinely, and unsafe retry can
duplicate provider cost or produce conflicting memo drafts.

**Independent Test**: Run the use case against deterministic backend and source
audio doubles that simulate offline submission, progress, timeout, rate limiting,
temporary failure, restart recovery, and eventual completion. Verify every retry
preserves the original operation and source identities.

**Acceptance Scenarios**:

1. **Given** the device is offline before any bytes are accepted, **When** the
   user submits, **Then** the operation remains recoverable with the same identity
   and no backend operation is claimed to exist.
2. **Given** upload progress is available, **When** bytes are transferred,
   **Then** observers receive monotonic, sanitized progress without audio content
   or filesystem locations.
3. **Given** the create outcome is uncertain, **When** the user retries, **Then**
   the existing backend operation is resolved by status when its backend identity
   is known, or by replaying the identical idempotent create request when the
   original response and backend identity were both lost; neither path dispatches
   a second provider operation.
4. **Given** a retryable failure with retry guidance, **When** retry is attempted
   after the allowed delay, **Then** the same logical operation resumes without a
   new idempotency identity.
5. **Given** the app is terminated after acceptance, **When** the operation is
   recovered, **Then** its durable identity and last known state allow status
   resolution without relying on an in-memory session.

---

### User Story 3 - Cancel and contain sensitive data (Priority: P3)

A mobile user can cancel an unfinished transcription, receive a deterministic
terminal outcome, and trust that late results and sensitive data do not escape
the operation's privacy boundary.

**Why this priority**: Cancellation, cleanup, and content-safe diagnostics are
required to make voice uploads trustworthy and operationally supportable.

**Independent Test**: Cancel operations before upload, during upload, while
processing, and after a concurrent backend completion. Verify idempotent terminal
outcomes, late-result rejection, cleanup signaling, and content-free errors and
events.

**Acceptance Scenarios**:

1. **Given** an operation has not been accepted remotely, **When** it is
   cancelled, **Then** no upload begins and the local operation becomes
   cancelled.
2. **Given** upload or processing is active, **When** cancellation wins the
   terminal race, **Then** remote cancellation/deletion is requested once and a
   late transcript cannot replace the cancelled outcome.
3. **Given** completion wins the terminal race, **When** cancellation arrives,
   **Then** the stored completed result remains authoritative and the conflict is
   reported without deleting or replacing it implicitly.
4. **Given** remote cleanup is pending, **When** cancellation is repeated, **Then**
   the existing cleanup state is returned or advanced without creating a new
   operation.
5. **Given** any failure or progress event, **When** it is exposed outside the
   transcription boundary, **Then** it contains no audio, transcript text,
   credential, authorization value, provider payload, or local file location.

### Edge Cases

- The source artifact disappears or changes after validation but before or
  during upload.
- The source is zero bytes, over the advertised size or duration limit, uses an
  unsupported format, or has inconsistent media metadata.
- The backend accepts the operation but the response is lost.
- Two submit actions for the same source arrive concurrently.
- The same idempotency identity is accidentally paired with changed source
  content or transcription options.
- Progress callbacks arrive late, out of order, or after a terminal state.
- Status polling races with cancellation, completion, deletion, or expiry.
- The app moves to the background or is terminated during upload or polling.
- Authentication expires before submission, during upload, or while resolving
  an uncertain outcome.
- The backend returns malformed success data, an unknown state, an unknown error
  code, blank transcript text, or inconsistent retry guidance.
- Remote cancellation succeeds while local state persistence temporarily fails,
  or local cancellation is stored while the remote request outcome is uncertain.
- Cleanup remains pending beyond an expected retry and must stay observable
  without exposing a storage location.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The system MUST represent one transcription as a stable logical
  operation associated with exactly one finalized source-audio identity.
- **FR-002**: The operation identity and idempotency identity MUST be created
  before first submission and MUST remain unchanged across progress, polling,
  retry, restart recovery, cancellation, and terminal resolution.
- **FR-003**: The system MUST validate source identity, readability, supported
  media metadata, positive length and duration, and integrity metadata before
  attempting submission.
- **FR-004**: The system MUST obtain source audio only through a trusted
  application boundary and MUST NOT expose a raw local path to user-interface
  consumers, events, errors, or diagnostics.
- **FR-005**: Submission MUST use the versioned application-backend contract and
  MUST NOT communicate directly with a transcription provider.
- **FR-006**: The system MUST distinguish at least ready, waiting-for-network,
  uploading, queued, processing, completed, retryable-failure,
  terminal-failure, cancelling, cancelled, and cleanup-pending outcomes.
- **FR-007**: Only a completed operation with non-blank final text MAY expose an
  authoritative transcript; partial or streaming text MUST NOT become a final
  result.
- **FR-008**: Submission and status resolution MUST map backend operation states,
  failures, retry guidance, and correlation identity into stable product
  outcomes without exposing provider-specific fields.
- **FR-009**: Progress observations MUST be monotonic within one upload attempt,
  scoped to the stable operation identity, and ignored after a terminal outcome.
- **FR-010**: A repeated submission with unchanged source content and options
  MUST resolve the existing logical operation rather than create a second one.
- **FR-011**: Reuse of an idempotency identity with changed source content or
  options MUST fail without replacing the original operation.
- **FR-012**: After an uncertain submission or transport timeout, the system MUST
  resolve status using the known backend identity when available. If the response
  carrying that identity was lost, the only permitted resubmission is an exact
  replay with the same idempotency identity, source fingerprint, and options so
  the backend resolves the existing logical operation without another provider
  dispatch.
- **FR-013**: Automatic retry MUST be limited to failures explicitly classified
  as retryable, MUST honor safe retry guidance, and MUST use bounded delay and
  attempt policies; user-actionable and terminal failures MUST NOT retry
  automatically.
- **FR-014**: Offline submission MUST preserve a recoverable operation and source
  association without claiming remote acceptance.
- **FR-015**: Cancellation MUST be idempotent, preserve the first terminal winner,
  stop further local transfer when possible, request remote cancellation or
  deletion when applicable, and reject late completion after cancellation wins.
- **FR-016**: The current operation identity, state, terminal winner, retry
  eligibility, and non-content recovery metadata MUST be durably recoverable
  after application restart.
- **FR-017**: Malformed backend success data, unknown states, inconsistent
  identity, and blank final text MUST produce a stable terminal or uncertain
  failure rather than a successful transcript.
- **FR-018**: Thin external commands and events MUST validate and delegate only;
  all transition, idempotency, retry, cancellation, and late-result rules MUST
  have one application-level authority.
- **FR-019**: Automated tests MUST cover success, duplicate submission, progress,
  offline state, timeout, restart recovery, cancellation races, retryable and
  terminal failures, authentication expiry, malformed responses, and cleanup
  pending without live audio or a production backend.

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: iOS and Android MUST receive equivalent product states and error
  classifications for the same finalized media and backend response.
- **MLR-002**: Leaving the foreground MUST NOT silently create a second upload or
  continue an unsupported transfer; the operation MUST remain recoverable for
  explicit resume or status resolution.
- **MLR-003**: Application termination at any submission, upload, polling, or
  cancellation boundary MUST preserve enough non-content state to resolve the
  same operation after relaunch.
- **MLR-004**: Slow and changing networks MUST expose progress or a recoverable
  uncertain state without treating a missing response as rejection.
- **MLR-005**: This feature MUST NOT request microphone permission, change audio
  session behavior, record in the background, or add realtime transcription.
- **MLR-006**: Physical iPhone and Android validation MUST submit the same safe,
  deterministic finalized-audio fixture through a non-production backend test
  environment and demonstrate success plus one offline/timeout recovery flow.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

- **PDL-001**: Provider and backend credentials MUST remain outside the client
  feature's domain data, persisted operation data, events, errors, logs,
  analytics, and test fixtures.
- **PDL-002**: Audio bytes MAY be read only for validation and authorized upload;
  they MUST NOT be copied into operation state, logs, analytics, error payloads,
  or progress events.
- **PDL-003**: Transcript text MAY exist only in an authorized completed result
  and MUST be excluded from logs, analytics, error details, and correlation data.
- **PDL-004**: Local source audio MUST remain available while an operation is
  waiting for network, uncertain, or retryable; this feature MUST NOT decide
  post-memo retention on the user's behalf.
- **PDL-005**: Remote temporary content deletion and cleanup-pending outcomes MUST
  follow the backend contract and remain observable without exposing internal or
  signed locations.
- **PDL-006**: Cancellation or terminal failure MUST produce an explicit local
  and remote cleanup disposition; unresolved cleanup MUST remain retryable and
  traceable by non-content identifiers.
- **PDL-007**: The source-audio retention choice remains a user-visible decision
  owned by the later integrated journey; until that choice exists, successful
  transcription MUST NOT imply permanent local retention or deletion.

### Architecture Impact _(mandatory)_

- Rust domain concepts own product-level transcription identities, states,
  failures, retry rules, and terminal invariants without depending on the app
  shell, operating systems, files, transport clients, or provider SDKs.
- Application use cases orchestrate submission, status resolution, retry,
  cancellation, and restart recovery exclusively through outbound contracts for
  source-audio access, operation state, user authorization, and backend
  transcription.
- Infrastructure adapters own file access and backend transport details. The
  backend adapter conforms to the versioned contract from Issue #3 and never
  calls the transcription provider directly from the mobile client.
- Inbound app commands validate identifiers and delegate to application use
  cases; they contain no retry, state-transition, cleanup, or idempotency rules.
- React-facing integration in this feature is limited to a deliberate entity API
  for invoking and observing the use case. Remote operation state belongs to
  TanStack Query in the later UI integration; it MUST NOT be duplicated in
  Zustand.
- This feature defines no production recording controls or memo editor. Those
  compositions remain owned by Issues #6 and #7.

### Key Entities _(include if feature involves data)_

- **Transcription Operation**: One durable logical request, including stable
  operation/source identities, current product state, first terminal winner,
  retry eligibility, and safe correlation metadata.
- **Finalized Source Audio**: The immutable, readable recording selected for
  upload, described by opaque identity, verified media metadata, positive size
  and duration, and integrity information.
- **Transcription Result**: One non-blank final text associated with a completed
  operation, with optional provider-neutral language information.
- **Transcription Failure**: A stable content-free code and category indicating
  retryable, user-actionable, terminal, uncertain, or cancelled behavior plus
  optional safe retry guidance.
- **Upload Observation**: Ephemeral, operation-scoped progress or phase
  information that never contains source bytes, paths, transcript text, or
  credentials.
- **Cleanup Disposition**: The observable state of remote temporary-content
  deletion and any remaining retry obligation.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: In 100 deterministic success trials, one source identity produces
  exactly one logical operation and one non-blank final transcript, including
  repeated submit and status actions.
- **SC-002**: In 100 duplicate or uncertain-retry trials, no test creates a
  second operation identity or a second accepted transcription for the same
  source and options.
- **SC-003**: All tested failures map to a documented product category and safe
  next action, and zero error, event, log, or analytics assertions contain audio,
  transcript text, credentials, authorization values, provider payloads, or file
  locations.
- **SC-004**: Cancellation, completion, and late-result race tests preserve the
  first terminal winner in 100% of orderings and issue at most one effective
  remote terminal request.
- **SC-005**: After simulated termination at every non-terminal phase, the same
  operation can be resolved or safely retried within one user action after
  relaunch without relying on prior in-memory state.
- **SC-006**: On one physical iPhone and one physical Android device, a
  deterministic finalized-audio fixture completes transcription successfully,
  and one offline or uncertain-timeout scenario recovers without duplicate
  operation or transcript.
- **SC-007**: Automated domain and application tests run without microphone
  hardware, a production backend, or provider credentials and cover every
  documented operation state and terminal transition.

## Dependencies and Scope Boundaries

- Depends on the completed record-and-transcribe journey contract from Issue #2.
- Depends on the completed versioned backend transcription contract from Issue
  #3.
- Uses finalized source-audio descriptors compatible with the recorder contract
  from Issue #4, but does not connect recorder output to this use case; Issue #6
  owns that integration.
- Production backend implementation, provider dispatch, authentication UI,
  recording UI, transcript editing, memo persistence, background transfer,
  background recording, realtime transcription, and desktop release are out of
  scope.

## Assumptions

- A finalized source-audio item is already available through a trusted local
  boundary and has stable identity and integrity metadata.
- An authorized application session can be supplied through a replaceable
  boundary; this feature does not define sign-in or token refresh UX.
- The initial backend contract accepts the formats and limits published by Issue
  #3; the client does not infer provider capabilities independently.
- Network transfer runs only while the platform permits foreground work. A
  background transfer service requires a separate specification.
- A deterministic non-sensitive audio fixture and non-production backend double
  are available for automated and physical-device validation.
