# Contract: Backend-Mediated Transcription Boundary

## Purpose

Define the semantic client/backend boundary that Issue #3 will turn into a wire
contract. The mobile client never communicates with OpenAI directly.

## Logical operations

| Operation                       | Required semantic input                                                                           | Required result                                                                             |
| ------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Create or resolve transcription | Authenticated user context, stable operation ID, source-audio ID/integrity, verified audio upload | Existing or newly accepted logical operation                                                |
| Read status                     | Authenticated user context and operation ID                                                       | Queued, uploading, processing, completed, retryable failure, terminal failure, or cancelled |
| Cancel operation                | Authenticated user context and operation ID                                                       | Cancellation/late-result-ignore status and cleanup scheduled                                |
| Delete temporary data           | Authenticated user context and operation ID                                                       | App-controlled backend copy deleted or deletion pending with deadline                       |

Issue #3 owns endpoint paths, methods, authentication, signed-upload design,
payload schemas, and hard size/duration limits.

## Idempotency contract

- The application backend, not OpenAI or the client UI, is authoritative for
  idempotency.
- The idempotency key is the stable `TranscriptionOperationId` scoped to the
  authenticated user and immutable source-audio integrity.
- Reusing the same key with different source content is rejected.
- Reusing the same key with the same content returns the in-flight or terminal
  logical result and does not issue a second provider request.
- After an uncertain timeout, the client reads/resolves status before requesting
  another attempt.
- Provider request IDs are recorded as sanitized diagnostics but do not replace
  application idempotency.

## Result contract

- Only a non-blank final transcript can produce `completed`.
- Streaming/partial deltas, if ever proxied, are progress only and cannot be
  saved as the authoritative draft.
- The client does not select or receive an OpenAI model name.
- Provider-specific errors map to stable product categories with retry guidance;
  raw provider payloads are not exposed to clients or default logs.

## Security and privacy contract

- OpenAI credentials are loaded only by the backend from a secret manager or
  protected server environment.
- Audio submission requires authenticated, authorized user context and strict
  content size/type verification.
- Raw audio, transcript text, credentials, authorization headers, and signed
  upload locations are excluded from logs and analytics.
- App-controlled temporary backend audio is deleted within 24 hours of completed,
  cancelled, or terminal-failure processing.
- Cancellation invalidates queued work; late results are ignored and cleaned up.
- Provider data-use, retention, region, and model availability are verified in
  the production environment before enablement and recorded by Issue #3.

## Supported-content policy

The initial mobile preference is finalized m4a. Issue #3 selects the allowed
subset and hard limits using current provider documentation, validates actual
media content server-side, and returns limits to the client before recording.
The client must not hard-code a provider model or assume every provider-supported
format is accepted by the application.
