# Contract Design: Error Catalog

Every JSON error uses `application/problem+json` and the common `Problem`
schema. The OpenAPI contract contains one named example per code.

| HTTP | Code                      | Category        | Retryable         | Client action / dispatch rule                           |
| ---- | ------------------------- | --------------- | ----------------- | ------------------------------------------------------- |
| 400  | `MALFORMED_REQUEST`       | terminal        | No                | Fix request construction; no provider dispatch          |
| 401  | `AUTHENTICATION_REQUIRED` | user_actionable | No                | Refresh/sign in; no provider dispatch                   |
| 403  | `FEATURE_NOT_ALLOWED`     | user_actionable | No                | Explain account restriction; no provider dispatch       |
| 404  | `OPERATION_NOT_FOUND`     | terminal        | No                | Treat unknown and other-owner identically               |
| 409  | `OPERATION_CONFLICT`      | uncertain       | No automatic POST | GET known operation or retry after reservation guidance |
| 410  | `CONTENT_EXPIRED`         | terminal        | No                | Do not reprocess implicitly                             |
| 413  | `AUDIO_TOO_LARGE`         | user_actionable | No                | Record shorter audio; no provider dispatch              |
| 415  | `UNSUPPORTED_AUDIO`       | user_actionable | No                | Use supported finalized audio; no provider dispatch     |
| 422  | `AUDIO_DURATION_EXCEEDED` | user_actionable | No                | Record shorter audio; no provider dispatch              |
| 422  | `INVALID_LANGUAGE_HINT`   | user_actionable | No                | Remove/fix language hint; no provider dispatch          |
| 422  | `CHECKSUM_MISMATCH`       | terminal        | No                | Re-read source and create deliberate new operation      |
| 422  | `IDEMPOTENCY_MISMATCH`    | terminal        | No                | Never reuse key for changed content/options             |
| 429  | `RATE_LIMITED`            | retryable       | Yes               | Honor Retry-After with bounded backoff                  |
| 429  | `USAGE_LIMIT_EXCEEDED`    | user_actionable | No                | Wait for/reset/increase account allowance               |
| 500  | `INTERNAL_ERROR`          | uncertain       | No blind POST     | Resolve operation state before bounded retry            |
| 503  | `PROVIDER_UNAVAILABLE`    | retryable       | Yes               | Honor Retry-After; same logical operation               |
| 504  | `PROCESSING_TIMEOUT`      | uncertain       | No blind POST     | GET same operation; preserve idempotency identity       |

## Problem invariants

- `status` matches HTTP status; `code`, `category`, and `retryable` match this
  catalog.
- 429 and retryable 503 responses include both HTTP `Retry-After` and body
  `retry_after_seconds` with consistent integer seconds.
- `detail` and field-error reasons never echo submitted values.
- Problems never contain audio, transcript, provider response body, provider
  model, authorization, credential, multipart bytes, signed/internal URL, stack
  trace, or storage path.
- `request_id` is the backend correlation ID. Provider request IDs stay in
  protected operational metadata.
