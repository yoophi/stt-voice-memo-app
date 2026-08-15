# Feature Specification: Minimal Android Tauri Host

**Feature Branch**: `024-android-tauri-host`

**Created**: 2026-08-16

**Status**: Implemented — automated scope complete; physical acceptance remains Issue #23

**Input**: GitHub Issue #24 — initialize a minimal Android Tauri host for API 24+, restore the existing root Android workflow, and exclude unowned capabilities.

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Build the Android app from a clean checkout (Priority: P1)

A mobile contributor can prepare a clean checkout, use the documented repository-root workflow, and produce an installable Android application without manually creating or relocating native project files.

**Why this priority**: Every Android build, integration, and physical-device validation depends on a stable native host and repeatable root workflow.

**Independent Test**: Start from a clean checkout with the supported Android toolchain, follow the documented setup, run the root Android build, and confirm that the expected application artifact is produced from the stable native project path.

**Acceptance Scenarios**:

1. **Given** a clean checkout and supported Android toolchain, **When** the contributor runs the documented root build workflow, **Then** the native project is discovered without an interactive initialization step and an installable artifact is produced.
2. **Given** a machine missing a required Android toolchain component, **When** the contributor runs validation, **Then** the workflow stops with an actionable unavailable/failure reason and does not report Android as verified.
3. **Given** the initialized project, **When** repository validation runs, **Then** the Android minimum supported version remains API 24 and the existing iOS project remains unchanged.

---

### User Story 2 - Review a minimum-capability Android host (Priority: P2)

A security or mobile reviewer can inspect one explicit capability allowlist and confirm that the application host contains only the components required to launch the current foreground app shell.

**Why this priority**: Generated Android templates may include permissions, providers, television launchers, or lifecycle behavior that the current product does not own.

**Independent Test**: Compare the packaged Android manifest and native components against the documented allowlist and confirm that every unowned permission or component is rejected by automated validation.

**Acceptance Scenarios**:

1. **Given** the tracked Android host, **When** capability validation runs, **Then** it accepts the standard touch launcher and rejects microphone, storage, background-service, television, broad file-provider, and unreviewed network permissions or components.
2. **Given** a newly generated or edited manifest containing an unowned capability, **When** repository validation runs, **Then** it fails with the capability category and path without exposing configuration or secrets.
3. **Given** the Android app bundle, **When** client configuration inspection runs, **Then** no backend-only or provider credential name/value is present.

---

### User Story 3 - Hand off a launchable host for physical validation (Priority: P3)

A device tester can use the documented root workflow and evidence template to build, install, and foreground-launch the app on an Android API 24+ device in follow-up Issue #23.

**Why this priority**: The host is useful only if the project can progress to real-device validation without another initialization or capability-selection task.

**Independent Test**: Confirm that the validation guide names the exact build/install/launch workflow, expected foreground app-shell behavior, permission baseline, evidence fields, and owning follow-up issue.

**Acceptance Scenarios**:

1. **Given** a connected Android API 24+ device, **When** Issue #23 follows the handoff guide, **Then** no host generation or source modification is required before installation.
2. **Given** automated or emulator-only success, **When** completion evidence is reviewed, **Then** it is recorded separately and is not presented as physical-device acceptance.

### Edge Cases

- The native host exists only partially because initialization was interrupted.
- Regeneration reintroduces a television launcher, broad file provider, or permission outside the allowlist.
- The local SDK, JDK, NDK, Rust Android target, or device tooling is absent or points to an unsupported version.
- A contributor runs the Android command from a nested package instead of the repository root.
- Generated project identifiers or application identifiers drift from the tracked configuration.
- The app shell starts but a shared plugin reports an unsupported Android feature; the host must remain stable and the result must stay sanitized.
- Development-only connectivity needs are confused with permissions required by a packaged foreground app.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: The repository MUST contain one tracked Android native host at the stable path expected by the existing root mobile command.
- **FR-002**: A clean checkout MUST build the Android application through the documented repository-root workflow without an interactive host-generation step.
- **FR-003**: The Android application MUST retain API 24 as its minimum supported platform version.
- **FR-004**: The tracked host MUST preserve the current application identifier, app name, bundled frontend location, and shared mobile plugin registration.
- **FR-005**: The host MUST contain one standard touch-oriented launcher activity and MUST NOT contain a television launcher, Leanback declaration, or broad file-sharing provider.
- **FR-006**: The feature MUST define an explicit allowlist for Android permissions, exported components, intent filters, providers, services, and receivers; repository validation MUST fail closed on additions outside it.
- **FR-007**: The host MUST request no microphone, media/storage, notification, background execution, foreground-service, location, camera, contacts, or other sensitive runtime permission.
- **FR-008**: The packaged application and generated native configuration MUST contain no OpenAI credential, backend secret, authorization token, or backend-only configuration.
- **FR-009**: Validation MUST distinguish verified, unavailable, and invalid Android host states and MUST NOT report an absent or partial host as successful.
- **FR-010**: Contributor documentation MUST state which generated files are tracked, which SDK/build/signing outputs are ignored, how the host is reproducibly initialized, and how to run root build validation.
- **FR-011**: The implementation MUST preserve existing iOS source, signing configuration, recorder behavior, and frontend application behavior.
- **FR-012**: The physical Android build/install/foreground-launch evidence MUST be handed off to Issue #23 with an exact revision and content-safe evidence template; automated evidence MUST remain visibly distinct.

### Mobile and Lifecycle Requirements _(mandatory for affected features)_

- **MLR-001**: Android API 24+ foreground app-shell startup is the only new platform behavior in this feature.
- **MLR-002**: The host MUST NOT request audio permission, initialize an Android recorder adapter, start a foreground/background service, or continue work after the app is no longer visible.
- **MLR-003**: Missing toolchain, failed build, failed install, failed cold launch, and unsupported shared-plugin results MUST be reported as distinct, sanitized outcomes.
- **MLR-004**: Physical Android validation is owned by Issue #23 and remains incomplete until a real device records build, install, foreground launch, no-new-permission, and no-backend-configuration evidence.
- **MLR-005**: Desktop behavior, background recording, realtime transcription, Android microphone capture, and iOS lifecycle changes are deferred.

### Privacy and Data Lifecycle Requirements _(mandatory for audio/transcript features)_

- **PDL-001**: This host feature MUST NOT create, access, transmit, retain, or delete audio or transcript content.
- **PDL-002**: No provider/backend credential or sensitive configuration may be added to native resources, application metadata, build scripts, logs, evidence, or packaged assets.
- **PDL-003**: Validation fixtures and diagnostics MUST use names-only configuration checks or synthetic canaries and MUST never contain real secrets or user content.

### Architecture Impact _(mandatory)_

- No Rust domain, application, or port behavior changes are permitted; Android host wiring remains an infrastructure/composition concern.
- No React feature slice, entity state, TanStack Query state, or Zustand state is added.
- Kotlin is limited to the generated host and minimal application activity required to launch the existing Tauri application.
- Capability and path validation belongs to repository tooling, not application domain logic.

### Key Entities _(include if feature involves data)_

- **Android Host Baseline**: Stable project location, application identity, minimum platform version, launcher contract, and tracked-file policy.
- **Capability Allowlist**: The complete approved set of permissions and Android components; any unlisted item is invalid.
- **Build Evidence**: Content-safe record of revision, toolchain versions, command, artifact outcome, and whether validation was automated or physical.

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Three consecutive clean-checkout trials produce an installable Android artifact through the documented root workflow with no interactive host generation.
- **SC-002**: Automated inspection detects 100% of representative additions outside the capability allowlist and produces zero false success reports for absent or partial hosts.
- **SC-003**: The packaged host contains zero sensitive runtime permissions, zero unowned providers/services/receivers, and zero backend/provider secret findings.
- **SC-004**: All previously passing workspace, frontend, Rust, Swift, iOS-path, contract, and secret-boundary checks remain passing after host initialization.
- **SC-005**: Issue #23 can begin physical Android build/install/foreground-launch validation from the merged revision without generating or editing native source.
- **SC-006**: Missing or invalid Android toolchain trials return one actionable failure classification in every trial and never claim verification.

## Assumptions

- The repository-root `pnpm tauri android ...` CLI facade already exists; this feature supplies the missing native host/build path rather than creating a second command surface.
- The supported local baseline is Java 17, the configured Android SDK/NDK, Rust stable, and Android Rust targets already documented for contributors.
- Packaged foreground app-shell startup does not require an unowned sensitive permission; development-server connectivity is treated separately from release capability requirements.
- Physical-device execution and evidence completion remain in Issue #23 as explicitly requested, but this feature must leave that issue immediately executable.
- Microphone permission and an Android recorder adapter require their own feature specification.

## Dependencies

- Builds on the workspace implementation merged through PR #22.
- Unblocks Issue #23 physical Android workspace validation.
- Provides the Android host needed by the Android portions of Issues #5 and #10 without completing their product-specific validation.

## Out of Scope

- Android microphone permission, audio capture, recorder adapter, audio focus, or background/foreground recording service
- Backend runtime, authentication, provider integration, network API behavior, or OpenAI access
- Changes to iOS native behavior or desktop release support
- Physical-device evidence execution, which remains tracked by Issue #23
