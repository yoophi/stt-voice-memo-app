# Feature Specification: Backend Monorepo Workspace

**Feature Branch**: `011-backend-monorepo-workspace`

**Created**: 2026-08-15

**Status**: Draft

**Input**: GitHub Issue #11: "Establish the application-backend monorepo workspace"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Work in one clearly owned repository (Priority: P1)

A contributor can find the mobile application, backend application, shared wire
contract, Rust modules, validation scripts, tests, and future deployment assets
in one documented repository without guessing which area owns a change.

**Why this priority**: A stable ownership map and dependency direction are the
foundation for developing the backend without coupling it to mobile platform
code or exposing server concerns to the client.

**Independent Test**: Starting from a clean checkout, a contributor follows the
repository map and root instructions to locate each owned area, installs the
workspace once, and runs the documented mobile-only, backend-only,
contract-only, and full-repository checks without changing directories.

**Acceptance Scenarios**:

1. **Given** a clean checkout, **When** a contributor reads the repository map,
   **Then** every mobile, backend, shared-contract, Rust, test, script, generated,
   and deployment concern has one named owner and location.
2. **Given** workspace dependencies are installed, **When** a contributor runs a
   scoped root command, **Then** only the requested mobile, backend, or contract
   validation scope runs and returns a clear result.
3. **Given** the entire repository must be verified, **When** the full validation
   command runs, **Then** all applicable mobile, backend, contract, Rust, and
   native checks execute from the repository root.
4. **Given** a contributor adds a backend domain module or external adapter,
   **When** they follow the contributor guide, **Then** the new code follows the
   documented inward dependency direction and has an explicit owner.

---

### User Story 2 - Share one safe transcription contract (Priority: P2)

Mobile and backend contributors work from one canonical transcription wire
contract while keeping backend-only configuration and credentials inaccessible
to client source and client build output.

**Why this priority**: Contract duplication causes incompatible releases, while
secret leakage would invalidate the security boundary of the product.

**Independent Test**: Change a derived contract artifact without changing the
canonical contract and verify drift detection fails; restore derivation and
verify it passes. Populate unique canary values in backend-local development
configuration, build the client, and verify neither names intended only for the
backend nor canary values occur in client output.

**Acceptance Scenarios**:

1. **Given** the repository contains contract consumers, **When** a contributor
   identifies the source of a transcription field or state, **Then** it resolves
   to `contracts/transcription-api/v1/openapi.json` and not a competing contract.
2. **Given** a derived schema or client no longer matches the canonical contract,
   **When** contract validation runs, **Then** it fails with an actionable drift
   result before the change can be accepted.
3. **Given** backend-local configuration contains test-only secret canaries,
   **When** the mobile client is built and inspected, **Then** no backend
   credential value or backend-only configuration name appears in the output.
4. **Given** a contributor creates local backend configuration, **When** they use
   the provided template, **Then** only variable names and safe descriptions are
   versioned and no working credential is committed.

---

### User Story 3 - Validate only affected areas without losing full confidence (Priority: P3)

A maintainer receives focused validation for mobile, backend, or contract-only
changes while retaining a single full-repository validation path and preserving
existing iOS, Android, recorder, frontend, Rust, Swift, and contract behavior.

**Why this priority**: Focused checks shorten feedback, but the migration is only
valuable if existing mobile-first paths and release confidence remain intact.

**Independent Test**: Evaluate representative mobile-only, backend-only,
contract-only, and cross-cutting change sets and verify the expected validation
groups are selected. Then execute full validation and the documented mobile
project-path checks on iOS and Android targets.

**Acceptance Scenarios**:

1. **Given** only mobile-owned files change, **When** change validation is
   selected, **Then** mobile and applicable shared checks run without requiring
   backend runtime checks.
2. **Given** only backend-owned files change, **When** change validation is
   selected, **Then** backend and applicable shared checks run without requiring
   mobile native builds.
3. **Given** the canonical contract or a shared validation tool changes,
   **When** change validation is selected, **Then** both consumer-facing contract
   checks and all affected workspace checks run.
4. **Given** the repository migration is complete, **When** documented iOS and
   Android commands are run from the root, **Then** the existing Tauri mobile
   projects are discovered at stable paths and launch behavior is unchanged.

### Edge Cases

- A change touches both backend and mobile scopes, or changes a root manifest
  used by every workspace member.
- A renamed or moved file is absent from the changed-file set but still affects
  job selection.
- A generated artifact is missing, stale, or manually edited.
- Backend-local environment files exist beside similarly named client-safe
  configuration.
- A secret canary is transformed, encoded, minified, or embedded in a source map
  rather than appearing as plain text.
- A contributor runs a scoped command before all workspace dependencies are
  installed or while no production backend runtime exists yet.
- Mobile build tooling assumes the previous repository root or a relative Tauri
  project location.
- A backend module attempts to import a mobile package, an external adapter is
  imported inward, or client code imports backend runtime code.
- Full validation runs in an environment that cannot sign or launch physical
  mobile builds; automated path/build checks must remain distinct from recorded
  physical-device evidence.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The repository MUST define one documented monorepo map assigning
  ownership for mobile application code, backend application code, shared
  contracts, Rust crates, scripts, tests, generated artifacts, and future
  deployment assets.
- **FR-002**: The workspace MUST preserve an explicit dependency direction in
  which mobile and backend runtime areas may consume shared contracts, but may
  not import each other's runtime modules.
- **FR-003**: Backend domain and application areas MUST remain independent of
  transport frameworks, provider clients, persistence clients, queue clients,
  and deployment SDKs; those dependencies MUST enter through named adapters.
- **FR-004**: Root commands MUST provide mobile-only, backend-only,
  contract-only, and full-repository development, build, test, lint, and format
  workflows wherever the operation applies to that scope.
- **FR-005**: Root commands MUST fail clearly when a requested scope has no
  runnable application yet, rather than silently claiming that it was validated.
- **FR-006**: `contracts/transcription-api/v1/openapi.json` MUST be the single
  canonical transcription wire-contract source for mobile and backend consumers.
- **FR-007**: Any generated schema, type, fixture, or client derived from the
  canonical contract MUST be reproducible and MUST have an automated drift
  check that fails when committed output is stale or manually changed.
- **FR-008**: Backend-local environment templates MUST contain variable names,
  safe examples, and descriptions only; real credentials and secret-bearing
  local files MUST remain untracked.
- **FR-009**: Backend-only configuration names and values MUST NOT be exposed to
  mobile source modules, mobile runtime configuration, or mobile build output.
- **FR-010**: Automated validation MUST inspect representative mobile build
  output for backend and provider secret canaries, including transformed build
  artifacts where practical.
- **FR-011**: Change-scoped continuous integration MUST select the minimum mobile,
  backend, contract, and shared checks required by affected paths, while a full
  validation workflow remains available and documented.
- **FR-012**: Dependency caches MUST be scoped so one workspace area's cache
  cannot make another area's validation appear successful with stale outputs.
- **FR-013**: Existing frontend, Rust, Swift, recorder, and transcription-contract
  tests MUST remain runnable from documented root commands after migration.
- **FR-014**: Existing Tauri iOS and Android project locations, generation paths,
  and documented mobile commands MUST remain valid after migration.
- **FR-015**: Contributor documentation MUST explain installation, scoped and
  full workflows, dependency direction, configuration boundaries, canonical
  contract updates, generated-artifact handling, and how to add backend domain
  modules or adapters.
- **FR-016**: The migration MUST NOT select a production database, queue,
  authentication provider, cloud, deployment target, or transcription-provider
  SDK without a separately reviewed architecture decision.
- **FR-017**: Repository validation MUST identify forbidden cross-area imports,
  duplicate canonical contract files, and committed secret-bearing environment
  files with actionable failures.
- **FR-018**: Generated artifacts and caches MUST have documented ownership and
  version-control treatment so a clean checkout can reproduce required outputs.

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: The migration MUST NOT add mobile permissions, change recorder
  behavior, modify audio-session behavior, enable background recording, or add
  realtime transcription.
- **MLR-002**: iOS 15+ and Android API 24+ build targets and existing Tauri mobile
  project paths MUST remain unchanged and independently verifiable.
- **MLR-003**: Root mobile commands MUST preserve the current foreground app
  launch path for both platforms and MUST document when signing or a physical
  device is required.
- **MLR-004**: Physical iPhone and Android regression evidence is explicitly
  deferred to follow-up GitHub Issue
  [#23](https://github.com/yoophi/stt-voice-memo-app/issues/23). That issue MUST
  confirm the merged workspace can locate, build, install, and launch the app
  shell without adding permissions or exposing backend configuration; executing
  those device trials is outside this implementation PR's scope.
- **MLR-005**: CI and local checks that do not launch physical devices MUST be
  reported as automated migration evidence, not as substitutes for physical
  mobile acceptance.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

- **PDL-001**: This feature MUST NOT create, transmit, retain, transform, or
  delete audio or transcript content because it adds workspace infrastructure
  only.
- **PDL-002**: OpenAI credentials, backend credentials, authorization tokens,
  audio, and transcript text MUST remain absent from repository examples,
  generated outputs, caches, default logs, and validation artifacts.
- **PDL-003**: Secret-leak validation MUST use clearly synthetic canaries and
  MUST delete or exclude temporary canary-bearing outputs after validation.
- **PDL-004**: Client-safe configuration and backend-only configuration MUST have
  separate documented namespaces and ownership boundaries.

### Architecture Impact _(mandatory)_

- The repository boundary changes, but existing mobile Rust domain/application,
  ports, inbound adapters, infrastructure adapters, React slices, TanStack Query
  state, and Zustand state ownership remain behaviorally unchanged.
- A backend application area is reserved with the same inward dependency rule:
  pure domain and application behavior at the center, explicit outbound ports,
  and separately owned inbound and infrastructure adapters.
- The canonical OpenAPI document is shared as a wire contract only. Mobile and
  backend runtime modules do not become a shared business-logic package.
- This feature introduces no new remote asynchronous product state and no new
  client-only state, so it requires no TanStack Query cache or Zustand store.
- Production handlers, persistence, authentication, audio storage, queue workers,
  provider calls, and deployment implementation remain separately specified work.

### Key Entities _(include if feature involves data)_

- **Workspace Area**: A named repository scope with one owner, dependency rules,
  source locations, commands, and validation responsibilities.
- **Canonical Contract**: The single versioned transcription wire definition
  from which consumer-facing schemas and checks are derived.
- **Derived Artifact**: A reproducible schema, type, fixture, or client output
  tied to one canonical contract revision and protected by drift validation.
- **Configuration Class**: A client-safe or backend-only set of named settings
  with distinct exposure, storage, and version-control rules.
- **Validation Scope**: A mobile, backend, contract, shared, or full check group
  selected from affected paths and independently reportable.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: A new contributor can locate the owner and permitted dependency
  direction for every documented repository area in under five minutes.
- **SC-002**: Mobile-only, backend-only, contract-only, and full-repository root
  validation commands each complete with an unambiguous pass, fail, or
  intentionally unavailable result on a clean checkout.
- **SC-003**: In 100% of drift trials, modifying a derived contract artifact
  without updating it from the canonical source causes contract validation to
  fail before acceptance.
- **SC-004**: In 100% of synthetic canary trials, client validation detects any
  backend/provider secret value or backend-only configuration name present in
  mobile build output, while a clean build produces zero findings.
- **SC-005**: Representative mobile-only, backend-only, contract-only, and
  cross-cutting changes select all required validation groups and no unrelated
  platform build group in the four-path selection matrix.
- **SC-006**: The full root validation path preserves a 100% pass rate for all
  previously passing frontend, Rust, Swift, recorder, and contract checks.
- **SC-007 (deferred release gate)**: Follow-up Issue
  [#23](https://github.com/yoophi/stt-voice-memo-app/issues/23) records one
  physical iPhone and one physical Android device build, install, and launch of
  the merged app shell with no newly requested permission and no backend-only
  configuration. This criterion is not claimed by or included in PR #22.
- **SC-008**: A clean checkout can reproduce every committed derived artifact
  and yields zero uncommitted drift after generation and validation.

## Assumptions

- Issue #3's merged OpenAPI document is the canonical v1 transcription contract.
- Existing mobile source may move beneath a dedicated workspace area only if its
  Tauri mobile paths and commands remain compatible; minimizing movement is the
  default when the same ownership map can be achieved without relocation.
- The backend area may initially contain only architecture boundaries, workspace
  metadata, safe configuration templates, and validation scaffolding; it does
  not need a production server to satisfy this infrastructure issue.
- Local contributors use the repository's pinned package manager and supported
  Rust toolchain; CI runner and cache-provider selection is an implementation
  planning decision.
- Physical-device validation requires locally available signing and connected
  devices. It is excluded from PR #22 and tracked separately by GitHub Issue
  [#23](https://github.com/yoophi/stt-voice-memo-app/issues/23), without treating
  automated checks as substitute evidence.
- Existing recording and transcription behaviors are regression constraints,
  not implementation scope for this feature.

## Dependencies

- The merged backend transcription contract from Issue #3.
- The merged mobile recorder and transcription upload work, including PR #9 and
  the current `src-tauri` workspace behavior.

## Out of Scope

- Production HTTP handlers, database schemas, authentication implementation,
  audio storage, queue workers, OpenAI calls, and production deployment.
- Selecting a database, queue, authentication provider, cloud provider,
  deployment platform, or OpenAI SDK.
- Changing the mobile recording, transcription, memo, background, realtime, or
  desktop product behavior.
