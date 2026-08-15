# Feature Specification: Backend Transcription API Contract

**Feature Branch**: `003-backend-transcription-api`

**Created**: 2026-08-15

**Status**: Implemented

**Input**: GitHub Issue #3: "Define the backend transcription API contract"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Submit audio and retrieve a final transcript (Priority: P1)

A signed-in mobile user explicitly submits one finalized recording. The
application backend accepts it quickly, processes it without exposing provider
details, and lets the app retrieve one final transcript for that recording.

**Why this priority**: This is the smallest backend contract that unlocks the
record-to-edit journey while keeping provider credentials out of the client.

**Independent Test**: Against a contract-test double, submit an allowed m4a file
with valid authentication and operation identity, observe acceptance, then read
the operation until it returns one non-empty final transcript. No OpenAI request
or credential is used by the test.

**Acceptance Scenarios**:

1. **Given** an authenticated user, a finalized allowed recording, and a new
   operation identity, **When** the app submits the recording, **Then** the
   backend durably accepts one asynchronous operation and returns its status
   location without waiting for provider completion.
2. **Given** an accepted operation is still queued or processing, **When** the
   app reads its status, **Then** the response identifies the non-terminal state
   and provides appropriate polling guidance without partial transcript text.
3. **Given** processing produced usable final text, **When** the app reads the
   operation, **Then** it receives exactly one final transcript associated with
   the original operation and no provider credential or model identity.

---

### User Story 2 - Retry or cancel safely (Priority: P2)

A mobile user experiences a slow connection, timeout, duplicate submission,
rate limit, provider outage, or cancellation. The app can determine what
happened and recover without unnecessarily issuing a second provider request or
creating a duplicate memo result.

**Why this priority**: Mobile networks frequently produce uncertain outcomes;
unsafe retries can lose user trust and create duplicate cost.

**Independent Test**: Re-submit the same operation and audio, submit conflicting
audio under the same identity, simulate each error class, and cancel queued and
processing operations. The contract must return the same logical result for a
valid retry, reject a conflict, expose stable recovery metadata, and ignore any
late result after cancellation.

**Acceptance Scenarios**:

1. **Given** a request outcome is unknown, **When** the same user retries with
   the same operation identity and same audio integrity, **Then** the backend
   returns the existing operation and does not dispatch another provider request.
2. **Given** an operation identity was already used with different audio or
   options, **When** it is reused, **Then** the backend rejects the conflict and
   does not replace or reprocess the original operation.
3. **Given** an operation fails, **When** the app reads the typed failure,
   **Then** it can distinguish retryable, user-actionable, terminal, and uncertain
   failures while treating cancellation as an operation state, and determine the
   permitted next action.
4. **Given** an unfinished operation, **When** the authenticated owner cancels
   it, **Then** queued work is invalidated, app-controlled temporary content is
   scheduled for deletion, and late provider output cannot become a transcript
   result.

---

### User Story 3 - Enforce privacy and usage boundaries (Priority: P3)

The product operator and mobile user can trust that only an authorized owner can
submit or read a recording, abusive or accidental usage is bounded, sensitive
content is absent from default telemetry, and temporary backend content is
deleted predictably.

**Why this priority**: Voice content is sensitive and provider calls have direct
cost. Security, privacy, and usage control are release gates, not optional
operational enhancements.

**Independent Test**: Exercise unauthenticated, cross-user, oversized,
unsupported, over-duration, over-rate, cancellation, terminal failure, and
explicit deletion examples against the versioned contract. Verify rejection
occurs before provider dispatch and that responses/logging fields contain no
audio, transcript, credentials, or signed locations except where final text is
the explicitly authorized response payload.

**Acceptance Scenarios**:

1. **Given** missing, invalid, expired, or unauthorized user credentials,
   **When** any operation is attempted, **Then** access is rejected before audio
   is dispatched to the provider and no existence information leaks across users.
2. **Given** a user exceeds a documented request, concurrency, or usage limit,
   **When** another operation is requested, **Then** it is rejected with stable
   retry or user-action guidance and no provider dispatch occurs.
3. **Given** an operation reaches completed, cancelled, or terminal-failure
   state, **When** its cleanup deadline arrives, **Then** app-controlled audio
   and transcript content are deleted while non-content idempotency metadata may
   remain for the documented deduplication period.
4. **Given** a completed operation, **When** the owner explicitly deletes it
   after saving the memo, **Then** content becomes unavailable immediately and
   cleanup completion is observable without recreating the operation on retry.

### Edge Cases

- The multipart body is missing audio, contains multiple audio parts, claims a
  different media type than its verified content, or is truncated during upload.
- Audio is exactly at or one byte above the size limit, or exactly at or just
  above the duration limit.
- A language hint is absent, valid, unsupported, or syntactically invalid.
- Two identical requests arrive concurrently before an idempotency record is
  visible to either caller.
- The first response is lost after the upload is accepted, and the same request
  is retried from a relaunched app.
- A status read races with completion, cancellation, content deletion, or expiry.
- Provider work completes after logical cancellation or after the client deletes
  the operation.
- A retry occurs after transcript/audio content expired but while non-content
  deduplication metadata remains.
- Rate limits apply during an ongoing operation without converting the accepted
  operation into a new one.
- Cleanup temporarily fails and must remain observable and retryable without
  exposing sensitive file locations.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The contract MUST be explicitly versioned and define authenticated
  operations to create, read, cancel, and delete a transcription operation.
- **FR-002**: Creation MUST accept exactly one finalized audio file plus a stable
  client-generated operation identity using multipart upload.
- **FR-003**: The contract MUST accept only verified `m4a`, `mp3`, `mp4`, `mpeg`,
  `mpga`, `wav`, or `webm` audio up to 25,000,000 bytes and 10 minutes duration;
  declared metadata alone MUST NOT establish validity.
- **FR-004**: Creation MUST be asynchronous: successful durable acceptance
  returns before provider completion and exposes a location for the operation.
- **FR-005**: Operation reads MUST represent `queued`, `processing`, `completed`,
  `failed`, `cancelled`, `deleting`, and `deleted` states with valid terminal and
  non-terminal transitions.
- **FR-006**: Only `completed` operations MAY include a non-empty final transcript;
  partial provider output MUST NOT be exposed as an authoritative result.
- **FR-007**: The mobile contract MUST support an optional language hint but MUST
  NOT accept or reveal an OpenAI model name; provider/model selection is a
  backend deployment concern.
- **FR-008**: Every create request MUST carry a stable idempotency key scoped to
  the authenticated user, source-audio integrity, and transcription-affecting
  options.
- **FR-009**: A retry with the same key and fingerprint MUST resolve the existing
  logical operation; the same key with a different fingerprint MUST fail without
  provider dispatch.
- **FR-010**: Cancellation MUST invalidate queued work, prevent a late result
  from becoming client-visible, and schedule app-controlled content deletion.
- **FR-011**: Explicit owner deletion MUST make content unavailable immediately,
  expose deletion progress when cleanup is pending, and preserve enough
  non-content metadata to prevent accidental reprocessing.
- **FR-012**: Success and error payloads MUST include a backend request identifier
  suitable for support correlation without embedding user content.
- **FR-013**: Error payloads MUST use stable codes and explicitly classify each
  failure as retryable, user-actionable, terminal, or uncertain, including any
  safe retry delay; cancellation remains an operation state rather than an error
  category.
- **FR-014**: Authentication failures, authorization failures, invalid input,
  conflicting idempotency, excessive size/duration, unsupported media, rate
  limits, provider unavailability, processing timeout, cancellation, deletion,
  and expired content MUST each have a documented example.
- **FR-015**: The backend MUST stop accepting an incomplete upload after 120
  seconds and MUST ensure provider processing reaches a terminal contract state
  within 10 minutes of durable acceptance.

### Authentication and Usage Requirements

- **AUR-001**: All operations MUST require a user-scoped Bearer access token;
  operation ownership MUST be derived from authenticated context rather than a
  client-supplied user identifier.
- **AUR-002**: Unknown and other-user operation identifiers MUST produce the same
  not-found outcome so resource existence is not disclosed.
- **AUR-003**: A user MAY create at most 10 transcription operations per rolling
  minute and have at most 3 non-terminal operations concurrently.
- **AUR-004**: A user MAY read, cancel, or delete operations at most 60 times per
  rolling minute; rate-limited responses MUST include retry guidance.
- **AUR-005**: Account-level daily usage limits MUST be enforceable before
  provider dispatch and return a user-actionable limit outcome distinct from a
  temporary rate limit.

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: The same contract MUST serve iOS and Android; platform file paths,
  native permission state, and native audio APIs MUST NOT cross the boundary.
- **MLR-002**: A mobile app may recover after upload timeout or termination by
  reading or resubmitting the same operation identity; recovery MUST NOT require
  an in-memory client session.
- **MLR-003**: Offline recording remains client-local and creates no backend
  operation until the user explicitly submits after connectivity returns.
- **MLR-004**: Slow-network behavior MUST preserve upload progress/cancellation
  at the client boundary and MUST NOT treat an unreceived create response as
  proof that the backend rejected the operation.
- **MLR-005**: Contract tests MUST validate equivalent iOS and Android request
  inputs, but physical-device audio lifecycle validation remains owned by native
  recorder and integration features because this feature adds no audio capture.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

| Artifact                  | Created                                                  | Retained                                                                            | Terminal behavior                                                                                              |
| ------------------------- | -------------------------------------------------------- | ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Multipart upload bytes    | During authenticated creation                            | Only while receiving/validating or until promoted to accepted temporary audio       | Delete immediately on rejection, truncation, or abandoned upload                                               |
| Accepted temporary audio  | After validation and durable operation acceptance        | Through queued/processing/retryable work                                            | Delete within 24 hours of completed, cancelled, or terminal failure; delete earlier on explicit owner deletion |
| Final transcript content  | On successful provider completion                        | Until client retrieval/deletion or 24 hours after completion, whichever comes first | Make unavailable immediately on deletion and physically remove within the same 24-hour deadline                |
| Idempotency metadata      | On first accepted key or protected in-flight reservation | Seven days after terminal outcome                                                   | Delete after deduplication window; contains no audio or transcript text                                        |
| Provider request metadata | At provider dispatch                                     | Per operational audit policy                                                        | Retain only opaque request ID, model/config version, timings, token/usage counts, and outcome; no content      |

- **PDL-001**: OpenAI credentials MUST exist only in protected backend runtime
  configuration and MUST NOT appear in this client-facing contract, responses,
  examples, logs, analytics, or test fixtures.
- **PDL-002**: Raw audio, transcript text, authorization tokens, multipart bodies,
  and signed/internal locations MUST be excluded from default logs, traces, error
  payloads, and analytics.
- **PDL-003**: Provider data-use, retention, processing region, and selected-model
  availability MUST be verified and recorded before production enablement.
- **PDL-004**: Cleanup failure MUST retain a non-content `deleting` signal and be
  retried until the 24-hour terminal deadline; sensitive locations MUST remain
  hidden.

### Architecture Impact _(mandatory)_

- Issue #3 defines a transport-neutral backend boundary and its OpenAPI
  representation. It does not choose the backend framework, database, cloud,
  authentication provider, queue, or deployment platform.
- Future Rust `domain` and `application` code consume product-level operation
  states and errors through a transcription port; HTTP, multipart, Bearer tokens,
  and provider details stay in infrastructure adapters.
- Future React entity APIs and TanStack Query own submission/status/cancel/delete
  remote state. Zustand MUST NOT mirror operation responses or durable status.
- This feature adds no production backend, OpenAI call, Tauri command, native
  permission, recorder, persistence, or UI. Those remain in Issues #4 through #7
  or a separately specified backend implementation feature.

### Key Entities _(include if feature involves data)_

- **Transcription Operation**: One user-owned, retry-safe logical request from
  durable acceptance through completed, failed, cancelled, or deleted outcome.
- **Submission Fingerprint**: A non-content digest of owner scope, source-audio
  integrity, and transcription-affecting options used to detect idempotency
  conflicts.
- **Transcription Result**: One non-empty final transcript attached to a completed
  operation and subject to terminal content deletion.
- **Problem Response**: A stable typed failure with classification, permitted
  action, request correlation, and optional retry delay.
- **Cleanup Record**: Non-content deletion state and deadline for app-controlled
  audio/transcript artifacts.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Contract validation covers 100% of documented operations, states,
  success examples, and defined error classes without making an OpenAI request.
- **SC-002**: In 100 concurrent or repeated same-key contract trials, one logical
  operation permits at most one provider dispatch and one final result.
- **SC-003**: In all conflict trials, a reused key with changed audio or options
  is rejected before provider dispatch and cannot alter the original operation.
- **SC-004**: Auth, ownership, rate, concurrency, and daily-usage rejection tests
  prevent provider dispatch in 100% of unauthorized or over-limit cases.
- **SC-005**: A conforming client can classify 100% of documented failures into
  retryable, user-actionable, terminal, or uncertain behavior using only the
  response contract, while handling cancellation as an operation state.
- **SC-006**: All accepted uploads receive durable acceptance or a typed failure
  within 120 seconds, and 95% of accepted operations reach a terminal state
  within 10 minutes.
- **SC-007**: App-controlled audio and transcript content is unavailable
  immediately after explicit deletion and physically removed within 24 hours of
  every terminal outcome in 100% of lifecycle tests.
- **SC-008**: Automated inspection finds zero provider/backend secrets and zero
  unapproved audio or transcript content in the contract, examples, test output,
  default telemetry definitions, and client-visible errors.

## Assumptions

- The application will have a user authentication service before production;
  this feature defines Bearer-token semantics without selecting that service.
- Direct multipart upload is appropriate for recordings no larger than
  25,000,000 bytes; resumable or object-storage signed uploads require a separate
  contract if future limits increase.
- The 10-minute duration cap is intentionally above the two-minute usability
  benchmark while bounding mobile upload cost and processing time.
- The app backend selects and pins the provider model. The mobile client may send
  an optional BCP 47 language hint but cannot select an OpenAI model.
- A seven-day non-content idempotency window prevents accidental duplicate cost
  after terminal content has been deleted; an expired operation returns a stable
  expiry outcome rather than silently starting new provider work.
- Provider streaming, realtime microphone transcription, speaker diarization,
  translation, background recording, memo persistence, and synchronization are
  outside this feature.
- No formal clarification question is required because Issue #3, Issue #2's
  boundary contract, and the constitution determine all material choices; exact
  backend technology remains deliberately deferred.
