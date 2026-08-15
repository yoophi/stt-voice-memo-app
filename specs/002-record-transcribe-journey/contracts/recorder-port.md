# Contract: Mobile Foreground Recorder Port

## Purpose

Define one technology-neutral recorder behavior for future Swift iOS and Kotlin
Android adapters. This is a semantic contract, not a Rust trait, TypeScript API,
or Tauri command signature.

## Operations

| Operation          | Inputs                                     | Success                                  | Defined failures                                                |
| ------------------ | ------------------------------------------ | ---------------------------------------- | --------------------------------------------------------------- |
| Inspect permission | None                                       | Current permission outcome               | Platform unavailable                                            |
| Request permission | User-initiated context                     | Granted or denied/restricted outcome     | Request unavailable/cancelled                                   |
| Start session      | New session ID and app-private destination | Capture active                           | Permission, active session, storage, audio-session/device error |
| Stop session       | Active session ID and reason               | One finalized source-audio descriptor    | Unknown/stale session, encoder/finalization error               |
| Cancel session     | Active or finalized session ID             | Capture stopped and deletion result      | Cleanup pending/failed                                          |
| Recover sessions   | None                                       | Durable unfinished/finalized descriptors | Corrupt/unreadable artifact reported individually               |

## Source-audio descriptor

The successful stop result contains an opaque source-audio identity, verified
media type, byte length, duration, integrity metadata, and retention state. A
native file path is adapter-private and is never sent directly to React or logs.

## Cross-platform behavior

| Concern             | iOS 15+                                          | Android API 24+                                                             | Shared product result                                  |
| ------------------- | ------------------------------------------------ | --------------------------------------------------------------------------- | ------------------------------------------------------ |
| Permission prompt   | System microphone authorization after Record tap | Runtime `RECORD_AUDIO` after Record tap                                     | Granted, denied, restricted/settings recovery          |
| Foreground boundary | Stop on scene entering background                | Stop on non-visible `ON_STOP`                                               | Finalize partial audio; no background continuation     |
| Interruption        | Audio-session interruption notification          | Recorder error/info and recording-configuration observation where available | Stop, label reason, never auto-resume                  |
| Route change        | Observe input route changes                      | Observe device/capture configuration where supported                        | Stop on input removal/change that risks integrity      |
| Process death       | Best-effort temp container/manifest recovery     | Best-effort temp container/manifest recovery                                | Recovered ready item or explicit unrecoverable outcome |

## Invariants

- Only visible user intent can start capture.
- Only one active recording exists application-wide.
- Stop/cancel are idempotent for a session ID.
- No recording continues after the product considers the app backgrounded.
- Native adapters emit sanitized reason codes, never raw audio or transcript data.
- Session metadata and temporary audio live in app-private storage.
- Recovery never triggers upload.
- The adapter requests only microphone and app-private file capabilities required
  by its own implementation feature.

## Deferred implementation ownership

- Issue #4 owns the minimal platform-neutral recorder port/lifecycle contract,
  its transition tests, the Swift iOS adapter, and its Tauri
  plugin/capabilities.
- Equivalent Android native implementation requires its own explicit task/scope;
  this contract must be used unchanged unless Issue #2 is amended.
- Issue #5 consumes the finalized recorder descriptor and owns the separate
  recording-file access, backend transcription port, and upload application
  transition tests. It does not redefine the recorder state machine.
- Issue #6 owns event reconciliation between native, Rust, and React layers.
