# Contract Design: Transcription API v1

## Common HTTP contract

- Base path: `/v1`
- Media types: `multipart/form-data` for create,
  `application/json` for success, `application/problem+json` for errors
- Authentication: user-scoped `Authorization: Bearer <token>` on every operation
- Response correlation: `X-Request-Id` on every response
- Cache: `Cache-Control: no-store` on every response containing operation,
  transcript, or problem data
- Unknown and other-owner IDs share the same 404 problem

## POST `/v1/transcriptions`

### Request headers

| Header            | Requirement                                             |
| ----------------- | ------------------------------------------------------- |
| `Authorization`   | Required Bearer access token                            |
| `Idempotency-Key` | Required opaque 20–128 printable ASCII characters       |
| `X-Audio-SHA256`  | Required lowercase 64-character hex integrity assertion |

### Multipart parts

| Part              | Type   | Requirement                                                                |
| ----------------- | ------ | -------------------------------------------------------------------------- |
| `audio`           | Binary | Exactly one file; verified allowed format, ≤25,000,000 bytes, ≤600 seconds |
| `source_audio_id` | String | Required opaque client source identity                                     |
| `language_hint`   | String | Optional valid BCP 47 tag                                                  |

No user ID, provider, model, prompt, response format, storage location, or API key
is accepted.

### Responses

| Status | Meaning                          | Headers/body                                 |
| ------ | -------------------------------- | -------------------------------------------- |
| 202    | New or replayed active operation | `Location`, `Retry-After: 2`, operation body |
| 200    | Replayed terminal operation      | Operation body; `Idempotency-Replayed: true` |
| Error  | See catalog                      | RFC 9457 problem body                        |

The server reserves the owner/key atomically before provider dispatch. The create
response arrives after upload validation and durable acceptance, not provider
completion. The backend stops reading an incomplete request at 120 seconds.

## GET `/v1/transcriptions/{operationId}`

| Status | Meaning                                              | Headers/body                                                                  |
| ------ | ---------------------------------------------------- | ----------------------------------------------------------------------------- |
| 200    | Current operation                                    | Active responses include `Retry-After: 2`; completed may include final result |
| 404    | Unknown or non-owned                                 | Identical problem shape                                                       |
| 410    | Known operation whose content/representation expired | Terminal expiry problem                                                       |
| 429    | Read limit exceeded                                  | `Retry-After` plus retryable problem                                          |

Clients resolve uncertain create/transport outcomes with GET before POST retry.
No partial transcript is returned.

## DELETE `/v1/transcriptions/{operationId}`

| Status | Meaning                                               | Headers/body                                |
| ------ | ----------------------------------------------------- | ------------------------------------------- |
| 202    | Cancellation accepted or cleanup pending              | Cancelled/deleting operation, `Retry-After` |
| 204    | Sensitive content already absent and cleanup complete | No body                                     |
| 404    | Unknown or non-owned                                  | Identical problem shape                     |
| 410    | Known tombstone after representation purge            | Expiry problem                              |

DELETE has no request body and is idempotent. It invalidates queued/processing
work, makes content unavailable to the client immediately, discards late provider
output, and schedules cleanup within the terminal deadline.

## Operation representation

```text
id, source_audio_id, state, created_at, updated_at,
result?, failure?, cleanup, links
```

- States: `queued`, `processing`, `completed`, `failed`, `cancelled`, `deleting`,
  `deleted`.
- `result` exists only for `completed` before deletion and contains final `text`
  plus optional provider-neutral detected `language`.
- `failure` exists only for `failed` and contains stable `code`, `category`,
  `retryable`, and optional retry guidance.
- `cleanup` contains state, availability, and optional deletion deadline without a
  file/internal URL.
- `links.self` is the only required link. Provider endpoints never appear.

## Usage controls

- Create: 10 operations per rolling minute per authenticated user.
- Concurrency: 3 non-terminal operations per user.
- GET/DELETE: combined 60 requests per rolling minute per user.
- Daily account usage policy is enforced before provider dispatch.
- Temporary rate limit is retryable and supplies Retry-After. Daily usage limit
  is user-actionable and not automatically retried.

## OpenAPI implementation location

The canonical machine-readable contract is implemented at
`contracts/transcription-api/v1/openapi.json`. It must encode all rows above,
reference only local component schemas, and include a success example plus every
error code from `error-catalog.md`.
