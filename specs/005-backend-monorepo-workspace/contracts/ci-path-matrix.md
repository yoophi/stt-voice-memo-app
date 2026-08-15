# CI Path Selection Contract

## Scope outputs

The selector returns booleans `mobile`, `backend`, and `contract` plus sorted,
content-free reason codes. Unknown/root-impacting changes fail safe to all three.

| Changed path class                                              | Mobile | Backend |                      Contract                       |
| --------------------------------------------------------------- | :----: | :-----: | :-------------------------------------------------: |
| Mobile source/config or `src-tauri/**`                          |  Yes   |   No    | Contract guard only when part of mobile validation  |
| `apps/backend/**`                                               |   No   |   Yes   | Contract guard only when part of backend validation |
| `contracts/**`                                                  |  Yes   |   Yes   |                         Yes                         |
| Root manifests/lockfiles, shared workspace scripts, CI workflow |  Yes   |   Yes   |                         Yes                         |
| Root governance/specification affecting ownership               |  Yes   |   Yes   |                         Yes                         |
| Mobile-only device evidence                                     |  Yes   |   No    |                         No                          |
| Backend-only documentation                                      |   No   |   Yes   |                         No                          |
| Unknown or empty input                                          |  Yes   |   Yes   |                         Yes                         |

## CI jobs

1. `changes` checks out full-enough history, computes the changed paths, executes
   the repository selector, and exports its three booleans.
2. `contract` runs when contract output is true and uses Node/pnpm cache keyed by
   the root lockfile and contract sources.
3. `backend` runs when backend output is true and uses a backend-specific Node
   and Rust cache key; it never receives production secrets.
4. `mobile` runs when mobile output is true and uses mobile-specific Node/Rust
   cache keys. Native signing/device checks remain documented physical evidence.
5. `full` is available by explicit workflow dispatch and runs `pnpm validate`.

Skipped jobs must remain visible as skipped; the required aggregate reports the
result of every selected job and does not treat a failed dependency as success.
