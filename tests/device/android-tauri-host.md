# Android Tauri Host Evidence

**Feature**: `specs/006-android-tauri-host/spec.md`

**Scope**: Issue #24 records automated host/build evidence only. Physical-device
execution is excluded from this change and owned by GitHub Issue
[#23](https://github.com/yoophi/stt-voice-memo-app/issues/23). `Not run` is not a
pass, and emulator/build evidence must never replace a physical row.

## Automated evidence

**Revision**: working tree based on `801639e` (record final commit in PR/Issue #23)
**Date**: 2026-08-16
**Environment**: macOS, Java 17, Android SDK 36, Build Tools 35.0.0, NDK 28.2.13676358

| Command                                 | Result | Content-safe evidence                                              |
| --------------------------------------- | ------ | ------------------------------------------------------------------ |
| `pnpm validate:android-host`            | Passed | source host and toolchain verified                                 |
| `pnpm test:workspace`                   | Passed | 34 workspace/path/capability tests                                 |
| `pnpm build:android`                    | Passed | ARM64 bundled debug APK, 169864126 bytes                           |
| `check-android-apk.mjs --variant debug` | Passed | app ID/API 24, zero sensitive permissions, launcher verified       |
| Three isolated tracked-snapshot builds  | Passed | independent source/output dirs; post-build Git diff stayed empty   |
| APK SHA-256                             | Passed | `b825f06a51aa02c1d408a21e37cf6265a96e9c0bce31e0f7bb70b9d36a114a9f` |

The initial APK attempt failed before Gradle project configuration because Tauri
selected Android Studio JBR 25.0.2. The root runner now selects Java 17 and the
reviewed NDK explicitly; the second build passed. No secret, audio, transcript,
signing path, or user content is recorded.

## Issue #23 physical Android rows

| Field                                       | Evidence |
| ------------------------------------------- | -------- |
| Merged commit                               | Not run  |
| Device model                                | Not run  |
| Android/API version (API 24+)               | Not run  |
| Root build command                          | Not run  |
| APK install                                 | Not run  |
| Five cold launches                          | Not run  |
| Foreground app-shell behavior               | Not run  |
| Runtime permission prompts                  | Not run  |
| Android settings permission inventory       | Not run  |
| Unsupported recorder result remains bounded | Not run  |
| Client secret-name scan                     | Not run  |
| Failure/recovery trial                      | Not run  |

## Completion rule

- [ ] Issue #23 checks out the exact merged Issue #24 revision without generating
      or editing Android source.
- [ ] An API 24+ physical device installs and cold-launches the bundled APK.
- [ ] No sensitive runtime permission prompt or unowned Android component appears.
- [ ] Unsupported recorder behavior is sanitized and does not terminate the host.
