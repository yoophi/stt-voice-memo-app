# Phase 0 Research: Backend Monorepo Workspace

## Decision 1: Keep the mobile package at the repository root

**Decision**: Treat the existing root package as `@stt-voice-memo/mobile` and add
`apps/backend` plus `contracts` as pnpm workspace members. Do not relocate `src`,
`src-tauri`, or generated native projects in this issue.

**Rationale**: pnpm requires a root `pnpm-workspace.yaml` and supports filtering
packages by exact name or directory. The workspace root package can remain a
project while additional members are added. Keeping the mobile paths stable
preserves Tauri's current `frontendDist`, native project discovery, historical
contract tests, and signing setup.

**Alternatives considered**:

- Move mobile to `apps/mobile`: conventional, but it rewrites generated Apple and
  Android project paths, historical evidence, and Tauri relative configuration
  without adding a second mobile application.
- Add a task orchestrator immediately: rejected because pnpm filters and small
  explicit scripts cover three packages and avoid another cache/config layer.

**Sources**: [pnpm workspaces](https://pnpm.io/workspaces), [pnpm filtering](https://pnpm.io/filtering), [Tauri Vite configuration](https://v2.tauri.app/start/frontend/vite/)

## Decision 2: Promote Cargo to a virtual root workspace

**Decision**: Add a repository-root virtual `Cargo.toml`, move the shared
`Cargo.lock` to the repository root, list the existing Tauri package and its
crates/plugins as members, and move the Swift patch declaration to the root.

**Rationale**: Cargo workspaces share one lockfile and target directory, and
package selection supports scoped `-p` and full `--workspace` commands. Cargo
only honors `[patch]` at the workspace root. A virtual workspace is appropriate
because neither the mobile binary nor a future backend crate should be the
repository's implicit primary Rust package.

**Alternatives considered**:

- Keep the nested `src-tauri` workspace and create another backend workspace:
  rejected because it fragments lockfiles, caches, dependency review, and the
  full validation command.
- Create a backend crate now: rejected because Issue #12 owns the first backend
  domain/application implementation and its language-level module decisions.

**Source**: [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

## Decision 3: Reserve the backend package without pretending a runtime exists

**Decision**: `apps/backend` contains a package command facade, safe environment
template, and architecture/ownership documentation only. Its `dev` command
returns an explicit unavailable result; build/test/lint/format validate the
scaffold and shared contracts without claiming a server was built.

**Rationale**: Issue #11 excludes handlers and runtime technology selection, but
Issue #12 needs a stable owned location. An explicit unavailable result satisfies
the root command contract more honestly than an empty long-running process or a
placeholder HTTP server.

**Alternatives considered**:

- Add a placeholder server: rejected as production-handler scope creep.
- Leave no backend workspace member: rejected because scoped commands and future
  module ownership would remain undefined.

## Decision 4: Package the canonical contract in place

**Decision**: Add `contracts/package.json` around the existing
`contracts/transcription-api/v1/openapi.json`. Generate only a deterministic
manifest containing contract version, path, and SHA-256. A check mode recomputes
the output and fails on drift.

**Rationale**: Consumers gain one named workspace dependency without copying the
OpenAPI file. A minimal generated manifest proves the generation/drift workflow
before Issue #16 adds generated wire types. SHA-256 and stable JSON ordering make
clean-checkout reproduction byte-for-byte testable.

**Alternatives considered**:

- Copy OpenAPI into mobile and backend packages: rejected because multiple
  canonical-looking files can drift.
- Introduce a full OpenAPI generator now: rejected because no backend framework
  or generated client target has been selected.

## Decision 5: Enforce boundaries with content-safe repository checks

**Decision**: Implement Node-based checks that (a) reject mobile imports of
`apps/backend`, backend imports of mobile runtime paths, and extra OpenAPI
canonical files; (b) derive backend-only environment names from
`apps/backend/.env.example`; and (c) scan mobile source and built assets for those
names plus caller-supplied synthetic canary values.

**Rationale**: These checks use the already pinned Node runtime, require no secret
scanner dependency, and can be unit-tested with temporary fixtures. Allowlisted
paths and stable error codes are easier to audit than broad regex logging.

**Alternatives considered**:

- Rely on `.gitignore`: rejected because ignored local files can still leak into
  build configuration or committed examples.
- Add production secret-scanning software: valuable later, but outside the
  minimal repository boundary and does not replace build-output inspection.

## Decision 6: Use one tested path classifier for local and CI selection

**Decision**: A pure classifier maps changed paths to `mobile`, `backend`, and
`contract` booleans. Root/shared tooling selects all scopes. Contract changes
select contract plus both consumers. GitHub Actions obtains the changed file list
and delegates selection to this script; `workflow_dispatch` can run full checks.

**Rationale**: One implementation prevents workflow YAML and local tests from
drifting. GitHub job-level conditions can use outputs from the classifier job.
Cache keys include runner, scope, and the appropriate lockfile/hash inputs so
one scope cannot certify stale output from another.

**Alternatives considered**:

- Separate workflow `paths` filters: simple but duplicates cross-cutting and
  contract dependency rules across workflow files.
- Third-party path-filter action: ergonomic, but unnecessary for this small
  auditable classifier.

**Sources**: [GitHub job conditions](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-jobs-with-conditions), [GitHub dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)

## Decision 7: Keep automated and physical mobile evidence separate

**Decision**: Automated validation checks that Tauri configuration, generated
Apple project, Android location rule, Rust package, and mobile commands remain
discoverable. Physical build/install/launch execution is excluded from PR #22
and transferred to GitHub Issue
[#23](https://github.com/yoophi/stt-voice-memo-app/issues/23), which updates the
device evidence document with no new permissions or backend configuration
observed.

**Rationale**: Repository CI generally cannot satisfy signing and device access.
The constitution requires a real-device verification plan for mobile regression
and forbids treating simulator or compile-only output as equivalent. A separate
tracked issue preserves that release gate without claiming it in this workspace
implementation PR.

**Source**: [Tauri environment and native project paths](https://v2.tauri.app/reference/environment-variables/)
