# Root Command Contract

All contributor commands start in the repository root. A command exits non-zero
on validation failure. An unavailable runtime command prints a stable reason and
exits with a distinct non-zero status; it must not print a success message.

| Scope    | Development                                          | Build                           | Test                 | Lint                 | Format/check           |
| -------- | ---------------------------------------------------- | ------------------------------- | -------------------- | -------------------- | ---------------------- |
| Mobile   | `pnpm dev:mobile`                                    | `pnpm build:mobile`             | `pnpm test:mobile`   | `pnpm lint:mobile`   | `pnpm format:mobile`   |
| Backend  | `pnpm dev:backend` (unavailable until runtime issue) | `pnpm build:backend` (scaffold) | `pnpm test:backend`  | `pnpm lint:backend`  | `pnpm format:backend`  |
| Contract | N/A with explicit message                            | `pnpm build:contract`           | `pnpm test:contract` | `pnpm lint:contract` | `pnpm format:contract` |
| Full     | `pnpm dev` aliases mobile                            | `pnpm build`                    | `pnpm test`          | `pnpm lint`          | `pnpm format:check`    |

Additional mandatory validation commands:

- `pnpm validate:mobile`: mobile TypeScript/build/test/lint plus Rust workspace
  package and native project-path checks appropriate to the host.
- `pnpm validate:backend`: backend scaffold/boundary/contract checks; it does not
  claim production handlers exist.
- `pnpm validate:contract`: canonical OpenAPI contract and generated drift checks.
- `pnpm validate`: all automated repository checks.
- `pnpm generate:contract`: write deterministic derived artifacts.
- `pnpm check:contract-drift`: check without modifying tracked files.
- `pnpm check:client-secrets`: inspect client sources and current `dist/` output.
- `pnpm select:scopes -- <paths...>`: print deterministic JSON selection.
- `pnpm tauri ios ...` and `pnpm tauri android ...`: unchanged mobile CLI
  facades. The Android facade exists, but its host/build remains unavailable
  until Issue #24 initializes the minimal reviewed project.

The root `test` command includes all currently runnable frontend, contract, Rust,
and Swift tests on supported hosts. Platform-specific omissions must be printed
and documented, not silently counted as passes.
