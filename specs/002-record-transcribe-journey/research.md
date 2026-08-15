# Research: Record and Transcribe Memo Journey

## Scope

This research resolves the product and architecture decisions required to make
Issue #2 a stable contract for Issues #3 through #7. It does not implement a
native recorder, backend, application use case, or production UI.

## Decision 1: Keep Issue #2 contract-only

**Decision**: Deliver the canonical behavior, state, data-lifecycle, port, and
device-validation contracts. Do not add microphone permissions, fake production
controls, native adapters, backend endpoints, Rust use cases, or memo UI.

**Rationale**: GitHub Issues #3 through #7 already own those implementations.
Adding partial runtime behavior here would break their dependency graph and the
foundation shell contract that forbids controls which appear functional before
their adapters exist.

**Alternatives considered**:

- A simulated recording UI was rejected because it would misrepresent product
  readiness and duplicate later UI work.
- Implementing the entire vertical slice was rejected because it would collapse
  five independently tracked implementation issues into a specification issue.

## Decision 2: Foreground-only means stop and finalize when not visible

**Decision**: Do not enable iOS background-audio capability or an Android
microphone foreground service. iOS finalizes an active recording on actual scene
background entry, not every transient inactive event. Android finalizes when the
activity/process becomes non-visible (`ON_STOP`), not merely paused. Neither
platform auto-resumes after interruption or backgrounding.

**Rationale**: Transient overlays should not unnecessarily terminate a visible
recording, while hidden recording is explicitly excluded. Background capture
would add platform capabilities, an Android persistent notification, review and
privacy implications, and a separate lifecycle contract.

**Alternatives considered**:

- Continuing in background was rejected as out of scope.
- Stopping on every temporary inactive/pause event was rejected because common
  system UI and multi-window transitions can trigger those events while the app
  remains meaningfully visible.

**Primary sources**:

- [Apple: Preparing your UI to run in the background](https://developer.apple.com/documentation/uikit/preparing-your-ui-to-run-in-the-background)
- [Apple: Managing your app's life cycle](https://developer.apple.com/documentation/uikit/managing-your-app-s-life-cycle)
- [Android: Activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle)
- [Android: Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)

## Decision 3: Request permission only from the Record action

**Decision**: Ask for microphone access only after the user taps Record. Treat
denied, restricted, one-time, revoked, and settings-only recovery as explicit
outcomes. Permission state is checked for every new recording attempt.

**Rationale**: This keeps permission contextual, supports Android one-time or
revoked grants, and avoids repeated prompts after denial.

**Alternatives considered**: Permission on launch was rejected because it lacks
user context and would make the non-recording app shell request a sensitive
capability prematurely.

**Primary sources**:

- [Apple: Requesting record permission](https://developer.apple.com/documentation/avfaudio/avaudiosession/requestrecordpermission%28_%3A%29)
- [Android: Request runtime permissions](https://developer.android.com/training/permissions/requesting)
- [Android: Explain access to sensitive information](https://developer.android.com/training/permissions/explaining-access)

## Decision 4: Interruptions finalize; they never auto-resume

**Decision**: A phone call, assistant, audio-session interruption, input-route
removal, encoder error, or capture contention stops the session. The adapter
finalizes usable partial audio when possible and returns a reason. The user must
explicitly choose to transcribe, discard, or start a new recording.

**Rationale**: Automatic resumption can record unexpectedly and obscures gaps in
the source. Android audio focus is a playback concept and is not sufficient as a
microphone-ownership signal.

**Alternatives considered**: Silent auto-resume was rejected for privacy and
recording-integrity reasons.

**Primary sources**:

- [Apple: Handling audio interruptions](https://developer.apple.com/documentation/avfaudio/handling-audio-interruptions)
- [Apple: Responding to audio route changes](https://developer.apple.com/documentation/avfaudio/responding-to-audio-route-changes)
- [Android: Sharing audio input](https://developer.android.com/media/platform/sharing-audio-input)
- [Android: MediaRecorder](https://developer.android.com/reference/android/media/MediaRecorder)

## Decision 5: Recovery is durable but best-effort during capture

**Decision**: A recorder adapter will create an app-private session manifest and
temporary source file at recording start. Orderly stop atomically marks finalized
audio ready. Relaunch inspects unfinished manifests and offers recovery or
deletion, but sudden termination during encoding is allowed to yield an explicit
unrecoverable result. Recovered audio is never uploaded automatically.

**Rationale**: Mobile processes can be terminated without a reliable destructor
or final callback. In-memory state alone cannot support the required relaunch
journey, and container finalization cannot always be guaranteed after a kill.

**Alternatives considered**: Promising complete partial recovery was rejected as
not achievable for every encoder/container and termination point.

**Primary sources**:

- [Apple: Scene disconnection](https://developer.apple.com/documentation/uikit/uiscenedelegate/scenediddisconnect%28_%3A%29)
- [Android: Processes and app lifecycle](https://developer.android.com/guide/components/activities/process-lifecycle)
- [Android: Saving UI states](https://developer.android.com/topic/libraries/architecture/saving-states)

## Decision 6: Provider details remain behind the backend

**Decision**: The client contract sends a supported finalized audio file to the
application backend and receives one final transcript result. The backend chooses
and pins the OpenAI transcription model. Mobile accepts no provider model name or
provider credential. `m4a` is the common mobile default; the backend validates
actual media type, size, and format against its current provider configuration.

**Rationale**: Current official OpenAI documentation supports `m4a` and other
common audio formats at the audio transcription endpoint and documents a 25 MB
file-guide limit. Model availability and limits can change independently of a
mobile release, so they belong in Issue #3's backend configuration and API
contract.

**Alternatives considered**:

- A client-selected model was rejected because it couples the app to provider
  operations and exposes unsupported choices.
- Realtime transcription was rejected because it is a separate session,
  streaming, reconciliation, and reconnect problem.

**Official OpenAI documentation**:

- [Speech to text guide](https://developers.openai.com/api/docs/guides/speech-to-text)
- [Create transcription API reference](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)

## Decision 7: Only a final result can become a draft

**Decision**: The MVP uses post-recording final-result semantics. If a future
backend streams file-transcription progress, deltas are display-only; only the
provider's final completion result can create an editable draft. An empty or
missing final result is a recoverable failure.

**Rationale**: Partial deltas can change and do not define a durable memo. This
keeps save, retry, and duplicate handling deterministic.

**Alternative considered**: Saving partial deltas was rejected because a retry
could produce conflicting memo content.

**Official OpenAI documentation**: [Streaming transcriptions](https://developers.openai.com/api/docs/guides/speech-to-text#streaming-transcriptions)

## Decision 8: Backend owns security, idempotency, and cleanup

**Decision**: OpenAI credentials live only in backend environment/KMS storage.
The application backend implements an idempotency key scoped to user and stable
transcription operation, persists in-flight and completed outcomes, and queries
that record before another provider attempt after an uncertain timeout. Provider
request IDs are diagnostic correlation only. Backend upload copies are deleted
within 24 hours of a terminal outcome.

**Rationale**: Official OpenAI transcription documentation does not promise
idempotent create behavior. Client request IDs are not a duplicate-prevention
contract. Current OpenAI data controls list no default application-state storage
for audio transcription, but that does not clean up application-controlled
client/backend files or logs.

**Alternatives considered**:

- Blindly retrying provider POST requests was rejected because it can duplicate
  cost and return different text.
- Treating provider data controls as the app's deletion policy was rejected
  because the app controls additional copies.

**Official OpenAI documentation**:

- [API authentication and request debugging](https://developers.openai.com/api/reference/overview)
- [Data controls and endpoint policies](https://developers.openai.com/api/docs/guides/your-data#default-usage-policies-by-endpoint)

## Decision 9: Architecture and state ownership

**Decision**: Future Rust implementation uses one domain journey aggregate and
ports for recorder, transcription, memo persistence, journey persistence, and
source-audio storage. Tauri commands remain thin inbound adapters. React uses
feature slices for actions, entity slices for domain-facing APIs/types, and a
widget/page for composition. Zustand owns only live capture UI. TanStack Query
owns durable recovered journeys, transcription operations, and persisted memos.

**Rationale**: This follows the constitution and the compatible patterns in
`~/project/agentic-workspace`. It prevents the whole workflow from accumulating
in one React hook and keeps durable recovery out of an in-memory store.

**Alternatives considered**:

- A single large workflow hook was rejected because it duplicates remote and
  durable state in the UI.
- A new shared workspace crate was rejected because there is currently one
  consumer; extract only when another real consumer exists.

## Device validation conclusion

Physical devices are mandatory. Android's official MediaRecorder guidance says
the emulator cannot record audio. Validation must cover at least one iPhone on
the supported baseline/current iOS range and one Android API 24/current Android
range, including permission, Home/app switch, interruption, route changes,
capture contention, force termination, low storage, offline retry, and deletion.
