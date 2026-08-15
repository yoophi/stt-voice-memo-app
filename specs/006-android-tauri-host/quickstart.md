# Quickstart: Android Tauri Host

## Prerequisites

From a fresh shell, verify:

```bash
java -version
echo "$ANDROID_HOME"
echo "$NDK_HOME"
adb version
rustup target list --installed
pnpm --version
```

The supported baseline is Java 17, Android SDK/Build-Tools/Platform-Tools,
side-by-side NDK `28.2.13676358`, and the `aarch64-linux-android` Rust target. Follow the
[official Tauri Android prerequisites](https://v2.tauri.app/start/prerequisites/#android)
when a component is missing.

## Validate the tracked host

```bash
pnpm validate:android-host
```

Success must explicitly report Android as `verified`. `unavailable`, `partial`,
and `invalid` are failures after this feature.

## Build an APK

```bash
pnpm build:android
```

The command builds the frontend and Rust Android target through Tauri and produces
a debug ARM64 APK beneath `src-tauri/gen/android/app/build/outputs/apk`.

For Play distribution in a separately owned signing workflow:

```bash
pnpm tauri android build --aab
```

## Regenerate only when intentionally updating the host

```bash
pnpm tauri android init --ci
```

Use the committed lockfile and review every generated diff. Re-apply and verify
the capability allowlist before accepting template changes. Normal builds never
need this command.

## Physical-device handoff

Issue #23 uses the merged revision and [device evidence template](../../tests/device/android-tauri-host.md)
to install and cold-launch on an API 24+ physical Android device. Do not mark its
rows complete from an emulator or APK-only build.

## Validation sequence

```bash
pnpm test:workspace
pnpm validate:android-host
pnpm build:android
pnpm validate:mobile
pnpm format:check
```

Record only revision, tool versions, stable result codes, artifact path, and
digest. Never record environment values, credentials, audio, or transcript text.
