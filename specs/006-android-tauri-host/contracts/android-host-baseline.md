# Android Host Baseline Contract

## Stable path and ownership

- The maintained host lives at `src-tauri/gen/android`.
- Host source, Gradle wrapper, build logic, manifest, Kotlin activity, and launcher
  resources are tracked.
- Gradle caches, compiled outputs, `local.properties`, IDE metadata, and signing
  material are ignored.
- Builds never initialize, delete, or regenerate the host.

## Identity and platform floor

- Tauri identifier and Android application ID: `com.yoophi.sttvoicememo`.
- Debug builds add `.debug` through the existing Tauri configuration.
- Product label remains `STT Voice Memo`.
- `src-tauri/tauri.conf.json` is authoritative for Android `minSdkVersion: 24`
  and frontend distribution path `../dist`.
- Gradle consumes those values without introducing a conflicting lower floor.

## Native activity

`MainActivity` extends `TauriActivity` and contains no additional initialization,
permission request, recording behavior, background work, or window/lifecycle
policy. Existing Rust plugin registration remains unchanged.

## Regeneration

From the repository root:

```bash
pnpm tauri android init --ci
```

Regeneration is a reviewed maintenance operation using the committed lockfile and
configuration. It is not part of a normal build and is not promised to be
byte-identical across Tauri CLI versions.
