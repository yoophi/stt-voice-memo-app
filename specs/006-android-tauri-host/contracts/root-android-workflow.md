# Root Android Workflow Contract

All commands run from the repository root and return non-zero on failure.

| Purpose                              | Command                          | Success evidence           |
| ------------------------------------ | -------------------------------- | -------------------------- |
| Source validation                    | `pnpm validate:android-host`     | stable `verified` summary  |
| Debug ARM64 APK                      | `pnpm build:android`             | installable debug APK path |
| Release AAB (later signing workflow) | `pnpm tauri android build --aab` | AAB output                 |
| Reviewed regeneration                | `pnpm tauri android init --ci`   | reviewed source diff       |

`build:android` invokes the project-pinned Tauri CLI with an explicit ARM64 target,
APK output, debug profile, and CI/non-interactive behavior. It does not sign a
release artifact or install to a device.

Validation distinguishes:

- host unavailable;
- host incomplete;
- host invalid;
- host verified;
- toolchain unavailable;
- build failed;
- artifact missing or policy invalid.

An absent toolchain never produces a verified or skipped-success result. Physical
install/launch is not part of this command contract and remains Issue #23.

`pnpm tauri android dev` is not an acceptance command because development-server
connectivity would require a separately reviewed debug-only network capability.
