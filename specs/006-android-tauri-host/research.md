# Phase 0 Research: Minimal Android Tauri Host

## Decision 1: Generate the standard host non-interactively

Use the repository-pinned CLI from the repository root:

```bash
pnpm tauri android init --ci
```

`android init` is Tauri's supported Android target initializer and `--ci`
suppresses prompts. The default target installation check remains enabled for
developer machines; `--skip-targets-install` is reserved for explicitly
pre-provisioned CI. The same lockfile and Tauri configuration are required for a
reviewable regeneration.

Sources: [Tauri Android CLI reference](https://v2.tauri.app/reference/cli/#android-init),
[Tauri Android CLI source](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/mobile/android/mod.rs)

Alternatives rejected:

- Android Studio New Project bypasses Tauri's Rust and Gradle wiring.
- A global/latest CLI makes template changes independent of the lockfile.
- Generating during every build would overwrite reviewed native customization.

## Decision 2: Track `src-tauri/gen/android`

The standard host path is `src-tauri/gen/android`. Commit Gradle scripts, wrapper,
build source, manifest, Kotlin activity, and resources. Ignore only caches,
compiled outputs, local SDK paths, IDE state, and signing material. Tauri's own
templates and signing documentation treat generated Android source as a maintained
project input.

Sources: [Tauri environment variables](https://v2.tauri.app/reference/environment-variables/),
[Android ignore template](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/templates/mobile/android/.gitignore),
[Android signing customization](https://v2.tauri.app/distribute/sign/android/)

Alternative rejected: ignoring all of `gen/android` would require interactive or
toolchain-dependent regeneration on clean checkout and discard legitimate native
review changes.

## Decision 3: Keep API 24 in Tauri configuration

`bundle.android.minSdkVersion` remains explicitly `24` in `tauri.conf.json`; the
generated Gradle host consumes this value. This avoids a second hand-maintained
platform floor. Android build tooling uses the Gradle value when present, and API
24 is Tauri's documented Android minimum.

Sources: [Tauri minimum Android version](https://v2.tauri.app/distribute/google-play/#changing-the-minimum-supported-android-version),
[Android manifest overview](https://developer.android.com/guide/topics/manifest/manifest-intro)

Alternative rejected: editing only generated `minSdk` creates split authority and
can drift on regeneration.

## Decision 4: Use an empty permission allowlist and one launcher

The current foreground shell needs no Android permission. The authored manifest
contains exactly one required touchscreen feature and one exported `MainActivity`
with `MAIN` and `LAUNCHER`. It has no app-owned providers, services, receivers,
activity aliases, television/Leanback declarations, or sensitive capabilities.
The launcher must be exported because an activity with an external launcher
intent filter must be startable by the system. The packaged manifest is checked
separately against reviewed AndroidX runtime components introduced by manifest
merging.

Sources: [Android manifest overview](https://developer.android.com/guide/topics/manifest/manifest-intro),
[Android activity `exported`](https://developer.android.com/guide/topics/manifest/activity-element),
[Android permission declaration](https://developer.android.com/guide/topics/manifest/uses-permission-element),
[Android manifest merger](https://developer.android.com/build/manage-manifests)

Alternatives rejected:

- Template `INTERNET` is not required by the packaged local shell and belongs to a
  later network-owning feature.
- FileProvider and broad file paths have no current product owner.
- Leanback changes device targeting beyond the mobile touch scope.
- `enableEdgeToEdge()` adds independent lifecycle/window behavior not needed for
  a minimal Tauri activity. Target SDK 36 still uses platform-enforced edge-to-edge
  behavior on newer Android; the existing safe-area CSS remains responsible for
  insets.

`tauri android dev` may require debug-only network access for a development server.
This feature does not add that overlay; it validates APKs built with bundled assets.

## Decision 5: Validate fail closed at source and package boundaries

The mobile-path validator has four Android classifications:

- `unavailable`: no host exists;
- `partial`: required host files are missing;
- `invalid`: identity, API floor, manifest, activity, or capability policy fails;
- `verified`: complete source baseline passes.

After this feature, repository validation requires `verified`; it no longer treats
absence as a successful partial state. Fixture tests mutate each forbidden category.
The APK build is additionally inspected with Android build tools so dependency
manifest merging cannot silently add a capability. The current AndroidX merge
adds one app-ID-scoped signature permission for non-exported dynamic receivers,
one non-exported startup provider, and one signature-guarded profile receiver;
these exact runtime inputs are allowed separately from the empty app-owned
permission set.

Alternative rejected: string checks against only the source manifest do not detect
partial projects or merged-manifest additions.

## Decision 6: Separate automated and physical evidence

This feature builds a debug ARM64 APK from the root and records content-safe
automated evidence. Issue #23 owns device installation, API-level confirmation,
cold/foreground launch, permission inspection, and failure/recovery trials. The
evidence template never promotes automated success to physical acceptance.

Source: [Tauri Google Play build guide](https://v2.tauri.app/distribute/google-play/)

## Toolchain baseline

- Java 17 compatible runtime
- Android SDK Platform, Platform-Tools, Build-Tools, Command-line Tools
- Side-by-side Android NDK `28.2.13676358` configured by `ANDROID_HOME`/`NDK_HOME`
- Rust Android targets, with ARM64 mandatory for this automated build
- project Gradle wrapper and lockfile-pinned pnpm/Tauri CLI

Source: [Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/#android)
