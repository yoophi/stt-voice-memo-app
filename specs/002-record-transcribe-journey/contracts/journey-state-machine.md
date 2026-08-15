# Contract: Recording-to-Memo Journey State Machine

## Purpose

This contract is the shared observable behavior for the native recorder, Rust
application layer, backend client, and React UI. Platform adapters may expose
additional diagnostics but may not invent different product states.

## Canonical states

| State                   | User-visible meaning                           | Allowed user actions                                   |
| ----------------------- | ---------------------------------------------- | ------------------------------------------------------ |
| `idle`                  | No active journey                              | Start recording                                        |
| `requesting_permission` | Waiting for system permission                  | Cancel request when platform permits                   |
| `permission_denied`     | Recording cannot start                         | Open settings/help; return to idle                     |
| `recording`             | Foreground audio capture active                | Stop; cancel                                           |
| `finalizing`            | Capture stopped; local file is being completed | Wait                                                   |
| `ready`                 | Finalized audio exists locally                 | Transcribe; delete                                     |
| `queued_offline`        | Submission waits for connectivity              | Retry when online; cancel/delete                       |
| `uploading`             | Backend upload is active                       | Cancel                                                 |
| `transcribing`          | Backend is producing a final result            | Cancel                                                 |
| `retryable_failure`     | Audio remains and same operation can resume    | Retry; delete                                          |
| `terminal_failure`      | Current attempt cannot continue                | Delete; start a deliberate new operation if applicable |
| `editable_draft`        | Final transcript can be edited                 | Edit; choose retention; save; discard                  |
| `saving`                | One memo commit is active                      | Wait                                                   |
| `saved`                 | Memo is committed                              | View memo                                              |
| `cancelled`             | Journey will produce no memo                   | Return to idle                                         |
| `unrecoverable`         | No usable finalized audio survived             | Acknowledge; return to idle                            |

## Event contract

| Event                   | Valid from                                        | Result / guard                                                                          |
| ----------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `start_requested`       | `idle`, `permission_denied` after settings change | Check/request permission; create one session only after grant                           |
| `permission_granted`    | `requesting_permission`                           | Enter `recording`                                                                       |
| `permission_denied`     | `requesting_permission`                           | Enter `permission_denied`; do not loop prompt                                           |
| `stop_requested`        | `recording`                                       | Enter `finalizing` exactly once                                                         |
| `backgrounded`          | `recording`                                       | Enter `finalizing` with background reason                                               |
| `interrupted`           | `recording`                                       | Enter `finalizing`; never auto-resume                                                   |
| `finalize_succeeded`    | `finalizing`                                      | Enter `ready` with one source-audio identity                                            |
| `finalize_failed`       | `finalizing`                                      | Enter `unrecoverable` with explicit reason                                              |
| `transcribe_requested`  | `ready`                                           | Enter `queued_offline` or `uploading` with one stable operation ID                      |
| `connectivity_restored` | `queued_offline`                                  | User-confirmed or policy-approved same-operation upload; no recovered-audio auto-upload |
| `upload_completed`      | `uploading`                                       | Enter `transcribing`                                                                    |
| `final_result_received` | `transcribing`                                    | Enter `editable_draft` only for non-blank final text                                    |
| `retryable_failure`     | `uploading`, `transcribing`                       | Enter `retryable_failure`, retain audio and operation ID                                |
| `terminal_failure`      | `uploading`, `transcribing`                       | Enter `terminal_failure`, retain audio pending decision                                 |
| `retry_requested`       | `retryable_failure`                               | Resolve backend state then reuse the same operation ID                                  |
| `save_requested`        | `editable_draft` with non-blank edited text       | Enter `saving` with one stable save identity                                            |
| `save_completed`        | `saving`                                          | Enter `saved`, then delete or retain audio per choice                                   |
| `cancel_requested`      | Any unfinished state                              | Confirm where destructive, invalidate work, enter `cancelled`                           |
| `late_result_received`  | `cancelled`, `saved`                              | Ignore; never recreate or duplicate content                                             |

## Duplicate action rules

- Repeated start returns the existing active-session state.
- Repeated stop returns the existing finalization/finalized result.
- Repeated transcribe/retry resolves the same `TranscriptionOperationId`.
- Repeated save resolves the same `MemoId`.
- UI controls are disabled or made idempotent while the corresponding transition
  is in progress; correctness does not depend on UI disabling alone.

## Relaunch rules

1. Load durable unfinished journey metadata before presenting an idle action.
2. If a readable finalized source exists, show `ready`, `queued_offline`,
   `retryable_failure`, or the backend-resolved active state.
3. If capture was active but the temporary container is readable, present it as
   interrupted recovered audio and require explicit submission.
4. If no usable audio survived, present `unrecoverable` once; never claim it was
   saved or upload it.
5. A cancelled journey remains cancelled even if a backend result arrives later.

## Error categories

| Category          | Examples                                                       | Recovery                                                            |
| ----------------- | -------------------------------------------------------------- | ------------------------------------------------------------------- |
| Permission        | Denied, restricted, revoked                                    | Settings/help; new start after grant                                |
| Capture           | Encoder, route removal, mic contention, no speech, low storage | Finalize partial when usable; otherwise explicit loss/new recording |
| Connectivity      | Offline, DNS, timeout before known response                    | Queue/status resolve/same-operation retry                           |
| Backend retryable | Rate limit, temporary service error                            | Same-operation retry after guidance                                 |
| Backend terminal  | Unsupported content, authentication/account unavailable        | Explain; do not loop retry                                          |
| Persistence       | Save or deletion failed                                        | Preserve source/draft and retry cleanup/commit safely               |
