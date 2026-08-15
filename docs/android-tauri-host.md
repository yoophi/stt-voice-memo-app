# Android Tauri Host

Issue #24 tracks a reviewed Tauri Android host at `src-tauri/gen/android`. A
normal build consumes this directory and never initializes or regenerates it.

## Tracked and ignored files

Tracked inputs include the Gradle wrapper and scripts, `buildSrc`, application
Gradle configuration, manifest, `MainActivity`, theme/string resources, and
launcher icons. Host-local ignore rules exclude only machine or derived state:
Gradle/IDE caches, build output, `local.properties`, signing material, generated
Tauri Gradle/config/assets/JNI files, and generated ProGuard inputs.

Do not add a blanket ignore for `src-tauri/gen/android`. The wrapper JAR is a
required reproducible build input.

## Toolchain

The root preflight requires:

- Java 17;
- Android platform 36 and Build Tools 35.0.0;
- Platform-Tools;
- side-by-side NDK `28.2.13676358`;
- all four Rust Android targets.

```bash
pnpm validate:android-host
```

The build runner selects the installed Java 17 on macOS and maps the existing
`ANDROID_HOME` SDK to NDK 28.2. It does not persist machine paths.

## Build and inspect

```bash
pnpm build:android
node scripts/workspace/check-android-apk.mjs \
  --apk src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk \
  --variant debug \
  --canary stt-apk-payload-canary-never-secret
```

The source host has no permission, one required touchscreen feature, and one
exported `MAIN`/`LAUNCHER` activity. The packaged APK is checked separately
because AndroidX manifest merging adds one application-scoped signature
permission, a non-exported startup provider, and a signature-guarded profile
receiver. Every merged member is exact-allowlisted; Android system permissions,
Leanback, FileProvider, services, and sensitive capabilities remain forbidden.
The validator also extracts the complete APK and scans assets, resources, native
libraries, and metadata for backend-only configuration names plus synthetic
canary representations. Findings are reported by category without echoing values.

`pnpm tauri android dev` is not an acceptance command in this feature. It may
need development-server network access, while the reviewed bundled APK owns no
`INTERNET` permission.

## Intentional regeneration

Only regenerate when reviewing a Tauri template update:

```bash
pnpm tauri android init --ci
```

Use the committed lockfile, review every diff, restore the minimum capability
contract, and rerun source plus packaged validation. Output is reviewable and
non-interactive but not promised to be byte-identical across CLI versions.

## Troubleshooting

### Gradle fails during `buildSrc` configuration with a Java version such as `25.0.2`

Tauri can prefer Android Studio's bundled JBR when `JAVA_HOME` is unset. The
current AGP/Gradle host requires Java 17. Use `pnpm build:android`; its runner
selects Java 17 explicitly. For a manual command:

```bash
export JAVA_HOME="$(/usr/libexec/java_home -v 17)"
export NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
pnpm tauri android build --debug --apk --target aarch64 --ci
```

### Tauri chooses the newest installed NDK

`ANDROID_NDK` is not sufficient for every Tauri/Gradle path. Set `NDK_HOME` to
the reviewed side-by-side version or use `pnpm build:android`, which supplies
both names to the child process.

### `apkanalyzer` cannot locate the latest Build Tools

Some Homebrew command-line-tools layouts do not look like Android Studio's
expected `cmdline-tools/latest` tree. The repository checker provides the SDK
tools-directory hint from `ANDROID_HOME` without printing the local path. Use
the root checker instead of copying an APK into an unconfigured shell.

## Physical device handoff

Issue [#23](https://github.com/yoophi/stt-voice-memo-app/issues/23) owns physical
API 24+ installation, cold launch, permission inspection, foreground behavior,
and bounded unsupported-recorder evidence. Issue #24 records build-only evidence
and must not mark physical rows complete.
