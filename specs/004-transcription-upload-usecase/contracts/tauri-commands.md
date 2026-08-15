# Contract: Transcription Tauri Commands and Events

## Command surface

Commands are async, validate public DTOs, delegate once to the application
service, and return sanitized camelCase DTOs.

| Command                 | Request                            | Result                                                                      |
| ----------------------- | ---------------------------------- | --------------------------------------------------------------------------- |
| `transcription_submit`  | `{ sourceAudioId, languageHint? }` | `OperationView`, optionally with final transcript only if already completed |
| `transcription_status`  | `{ operationId }`                  | authoritative `OperationView`                                               |
| `transcription_retry`   | `{ operationId }`                  | authoritative `OperationView`                                               |
| `transcription_cancel`  | `{ operationId }`                  | authoritative `OperationView`                                               |
| `transcription_recover` | `{}`                               | array of recoverable `OperationView` records                                |

The WebView cannot provide a backend URL, filesystem path, authorization token,
idempotency key, raw checksum, retry category, or target state.

## OperationView

Required safe fields:

```text
operationId, sourceAudioId, phase, attempt, updatedAtMs,
progress?, failure?, retryAtMs?, cleanup, backendRequestId?, transcript?
```

- `progress`: `{ suppliedBytes, totalBytes }`, advisory only.
- `failure`: stable `{ code, category, retryable }`; no raw detail.
- `transcript`: present only in an authorized completed command response; omitted
  from recover events and default operation persistence.
- Backend/provider URL, model, payload, token, path, signed locator, and audio are
  forbidden.

## Error DTO

Command errors contain only:

```text
code, category, retryable, operationId?, retryAtMs?, backendRequestId?
```

Messages are stable product text and never echo submitted values or transport
errors. Malformed identifiers and language hints fail before adapter dispatch.

## Advisory event

Event name: `transcription://event`

```text
eventId, operationId, sequence, attempt, phase,
progressBasisPoints?, failureCode?, retryAtMs?, cleanup
```

Rules:

- Event sequence increases per operation after durable commits.
- Upload progress is throttled and defined as bytes supplied to the HTTP client,
  not server acknowledgement.
- Transcript text is never emitted in an event.
- Consumers reconcile by requesting authoritative status and ignore older
  sequence/attempt values.
- Event delivery is advisory and may be missed across relaunch.

## Capability boundary

Expose only these five named commands through the app command surface. Do not add
generic HTTP or filesystem capability. Production transport accepts only the
configured HTTPS backend origin inside Rust infrastructure.

## Deferred integration

Issue #6 owns React entity APIs, TanStack Query mutations/status, recorder-source
registration, and end-user progress UI. Issue #5 may use a trusted diagnostic
harness for automated and device validation but adds no production controls.
