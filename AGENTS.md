# STT Voice Memo Agent Context

## Active Technology Baseline

- Mobile-first Tauri 2 application targeting iOS 15+ and Android API 24+.
- React 19, TypeScript 5, Vite, Tailwind CSS 4, shadcn/ui, TanStack Query 5,
  Zustand 5, and pnpm 11.
- Rust stable with edition 2024. Swift and Kotlin are limited to Tauri-generated
  mobile hosts or explicit native adapters.

## Architecture Rules

- Rust follows Hexagonal Architecture. `domain` and `application` do not depend
  on Tauri, operating-system APIs, persistence, filesystems, or network clients.
  External behavior enters through `inbound` and implements contracts through
  `infrastructure`; outbound contracts live in `ports`.
- React follows Feature-Sliced Design in the direction
  `app -> pages -> widgets -> features -> entities -> shared`. Lower layers do
  not import higher layers; slices expose deliberate public APIs.
- TanStack Query owns remote asynchronous state. Zustand owns only meaningful
  client-only state. Do not duplicate the same state or create empty global stores.

## Product and Security Rules

- iOS and Android physical-device behavior is the primary completion gate.
- Never ship OpenAI credentials or backend secrets in the Tauri client.
- Add mobile capabilities and runtime permissions only in the feature that uses
  them. The foundation app requests no sensitive permissions.
- Recording, transcription, memo persistence, authentication, backend calls,
  background recording, realtime transcription, and desktop release work require
  their own specifications.

## Current Feature

- Plan: `specs/001-tauri-app-init/plan.md`
- Specification: `specs/001-tauri-app-init/spec.md`
- Scope: reproducible React/Tauri foundation, architecture boundaries, developer
  validation, and physical iOS/Android app-shell verification.

## References

- Use `~/project/ext/handy` for voice-product behavior inspiration, not as the
  target architecture.
- Use `~/project/agentic-workspace` for established Tauri tooling and structural
  patterns where they conform to this project's constitution.
