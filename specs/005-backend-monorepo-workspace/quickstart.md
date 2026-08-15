# Quickstart: Validate the Backend Monorepo Workspace

## Prerequisites

- Node.js 22.22 or newer and pnpm 11.0.9 via Corepack
- Rust stable compatible with repository MSRV 1.85
- Xcode/Swift for Swift and iOS checks
- Android SDK/JDK for Android project checks
- Physical iPhone and Android devices plus signing for final device evidence

No backend, OpenAI, storage, queue, auth, or deployment credential is required.

## 1. Install and inspect ownership

```sh
corepack enable
pnpm install --frozen-lockfile
pnpm list -r --depth -1
cargo metadata --no-deps --format-version 1
```

Expected:

- the root mobile package, backend reservation, and contracts package are listed;
- one root Cargo workspace owns the Tauri package and existing Rust crates;
- `docs/monorepo-workspace.md` matches the boundary map in
  [workspace-boundaries.md](contracts/workspace-boundaries.md).

## 2. Run scoped checks

```sh
pnpm validate:contract
pnpm validate:backend
pnpm validate:mobile
```

`pnpm dev:backend` is expected to exit with the documented unavailable status
until a later backend-runtime issue implements a server. Other backend validation
checks validate only the scaffold and boundary.

## 3. Prove deterministic contract generation

```sh
pnpm generate:contract
pnpm check:contract-drift
git diff --exit-code -- contracts/transcription-api/v1/generated
```

For a negative trial, change a copy of the generated manifest in a temporary
directory and run the test fixture. Do not leave deliberate drift in the tree.

## 4. Prove the secret boundary

Build and scan the real client:

```sh
pnpm build:mobile
pnpm check:client-secrets
```

Automated tests create temporary client output containing synthetic canaries and
prove the scanner rejects them without printing their values. Never use a real
environment or credential value for this test.

## 5. Validate path selection

```sh
pnpm select:scopes -- src/app/App.tsx
pnpm select:scopes -- apps/backend/README.md
pnpm select:scopes -- contracts/transcription-api/v1/openapi.json
pnpm select:scopes -- package.json
```

Expected matrix:

| Trial              | Mobile | Backend | Contract |
| ------------------ | :----: | :-----: | :------: |
| mobile source      |  yes   |   no    |    no    |
| backend source     |   no   |   yes   |    no    |
| canonical contract |  yes   |   yes   |   yes    |
| root manifest      |  yes   |   yes   |   yes    |

## 6. Run full automated validation

```sh
pnpm validate
git diff --check
```

The command must include current frontend, contract, Rust, and Swift tests when
the host supports them. Any platform-specific omission is printed separately.

## 7. Check unchanged mobile project paths

```sh
test -d src-tauri/gen/apple/stt-voice-memo-app.xcodeproj
pnpm tauri ios build --debug --target aarch64 --no-sign
pnpm tauri android build --debug
```

If the Android project has not yet been generated in the checkout, use the
documented Tauri initialization command first and record that generated output
remains under `src-tauri/gen/android`. Do not commit local SDK/signing files.

## 8. Record physical-device evidence

Use `tests/device/backend-monorepo-workspace.md` to record, for each platform:

1. exact commit, device model, OS version, and root command;
2. build/install/foreground launch result;
3. unchanged permission prompts and recorder availability;
4. absence of backend-only configuration in the installed app/build inspection.

Automated, simulator, and unsigned-build results do not complete this gate.
