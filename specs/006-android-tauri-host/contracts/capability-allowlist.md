# Android Capability Allowlist Contract

## Authored and packaged permissions

The allowed permission set is empty. The allowed feature set contains only
`android.hardware.touchscreen` with `android:required="true"`. This includes normal, signature, runtime,
SDK-versioned, and custom permissions. In particular, the host must not request
network, microphone, storage/media, notification, camera, location, contact,
foreground service, wake lock, or background execution permissions.

## Components

Exactly one application component is allowed:

- activity `.MainActivity`;
- `android:exported="true"`;
- one intent filter containing action `android.intent.action.MAIN` and category
  `android.intent.category.LAUNCHER`.

Forbidden app-owned component kinds are providers, services, receivers, activity
aliases, instrumentation, and additional exported activities. Forbidden categories
and features include `LEANBACK_LAUNCHER` and `android.software.leanback`.

Android library manifests are merged into the packaged APK. Packaged validation
therefore maintains a separate exact list of reviewed AndroidX initialization and
profile components. A non-exported AndroidX startup provider or signature-guarded
profile receiver is not treated as an app-owned FileProvider, but every merged
component must still be named explicitly and fail closed on drift.

AndroidX Core also declares and uses one application-ID-scoped
`DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION` with signature protection. The packaged
allowlist accepts only this exact derived name and protection level; it is not a
system/runtime permission and cannot be replaced by a wildcard. All Android system
permissions remain forbidden.

## Validation behavior

- Validate required host paths before reading policy files.
- Parse XML semantics rather than accepting comments or loose substrings.
- Reject every unlisted permission, feature, component, action, or category.
- Inspect the built APK manifest after Gradle dependency merging.
- Report a stable category and repository-relative path; do not print manifest
  contents, credentials, environment values, or user data.
- No allowlist wildcard or unknown-value fallback is permitted.
