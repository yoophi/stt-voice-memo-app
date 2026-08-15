# Research: Transcription Upload Use Case

## Decision 1: Use a compile-isolated transcription core crate

**Decision**: Add `src-tauri/crates/transcription-core` with domain,
application, and outbound port modules. Root Tauri modules implement all external
adapters and composition.

**Rationale**: A separate workspace crate gives a compile-time boundary proving
that state and use-case rules do not import Tauri, operating-system APIs,
filesystem, persistence, HTTP, or OpenAI code. It matches the existing
`recorder-core` precedent without adding a platform plugin that this feature does
not need.

**Alternatives considered**: Root `src-tauri/src/domain` modules were rejected
because dependency isolation would be convention-only. A Tauri plugin was
rejected because there is no native iOS/Android API seam in this feature.

## Decision 2: Keep one application authority with object-safe async ports

**Decision**: `TranscriptionService` owns every state transition, retry decision,
terminal race, and sequence guard. It depends on object-safe async ports via
`async-trait` and shared `Arc` handles, with no mutex held across network awaits.

**Rationale**: Cancellation must be able to run while an upload is awaiting I/O.
Repository revision compare-and-swap prevents concurrent commands from replacing
the first terminal winner without serializing the whole network operation.

**Alternatives considered**: A single `Mutex<TranscriptionService>` held across
awaits would block cancellation and status. Duplicating state rules in HTTP and
Tauri adapters would violate the single-authority requirement.

## Decision 3: Preserve local and backend identities separately

**Decision**: Create a client `TranscriptionOperationId` before the first side
effect and use it as the idempotency key. Store a separate optional opaque
`BackendOperationId` only after a create response is observed.

**Rationale**: The local identity survives offline state and lost responses,
while the server identity is required for GET and DELETE. Conflating them would
invent behavior absent from the Issue #3 contract.

**Alternatives considered**: Treating the server ID as client-generated was
rejected because the contract defines it as server-owned. Using provider request
IDs was rejected because they are diagnostics, not idempotency.

## Decision 4: Resolve a lost create response by exact idempotent replay

**Decision**: When the backend ID is known, uncertain outcomes resolve by GET.
When the create response was lost before the backend ID was learned, retry
rebuilds and replays the exact multipart request with the same idempotency key,
source checksum, and normalized options. This is the only allowed uncertain POST
retry.

**Rationale**: Issue #3 exposes GET only by server ID; it has no lookup by client
operation or idempotency key. Exact POST replay is explicitly deduplicated by the
backend contract and cannot dispatch a second provider request, although audio
bytes may cross the network again.

**Alternatives considered**: Inventing an undocumented GET endpoint was rejected.
Amending the completed v1 contract can be considered later, but is not required
to implement its documented replay semantics.

## Decision 5: Stream multipart with explicit rustls transport

**Decision**: Use one reusable `reqwest 0.13` client with default features off and
`rustls`, `multipart`, `stream`, and `json` enabled. Configure HTTPS-only
production transport and a bounded connect/total timeout. Stream a trusted file
with `ReaderStream`, `Body::wrap_stream`, and `Part::stream_with_length`.

**Rationale**: Rust-side transport keeps backend URL and authorization out of the
WebView and bounds memory for 25 MB inputs. Explicit rustls avoids native
OpenSSL/native-tls variability across iOS and Android.

**Alternatives considered**: A frontend HTTP plugin would expose network/auth
surface to the WebView. Loading the whole file into memory was rejected. OS
background transfer was rejected as a separate lifecycle feature.

**Primary sources**:

- [reqwest multipart Part](https://docs.rs/reqwest/latest/reqwest/multipart/struct.Part.html)
- [reqwest Body streaming](https://docs.rs/reqwest/latest/reqwest/struct.Body.html)
- [reqwest ClientBuilder](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html)

## Decision 6: Define progress as advisory bytes supplied

**Decision**: Progress is attempt-scoped bytes supplied to the HTTP client, not
bytes acknowledged by the server. Sequence it monotonically, throttle to at most
10 Hz or percentage changes, and ignore stale/post-terminal updates.

**Rationale**: Reqwest and OS buffers cannot prove server receipt. The definition
is honest, deterministic, bounded, and sufficient for later UI feedback.

**Alternatives considered**: Claiming server-acknowledged progress was rejected as
unobservable. Phase-only progress is simpler but does not satisfy the issue's
progress contract.

## Decision 7: Use cancellation tokens plus remote reconciliation

**Decision**: Track operation-scoped `CancellationToken`s in infrastructure. A
cancel command records intent first, cancels local transfer, and then resolves or
issues idempotent DELETE. Dropping a request future is never treated as proof of
remote cancellation.

**Rationale**: Reqwest requests are futures without a remote cancel method.
Remote acceptance can race with local cancellation, so the backend resource and
first-terminal rule remain authoritative.

**Alternatives considered**: Aborting a task and immediately marking cancelled
was rejected because the backend may already be processing.

**Primary sources**:

- [Tokio CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
- [Tauri async commands and channels](https://v2.tauri.app/develop/calling-rust/)

## Decision 8: Use small atomic operation records, not a database

**Decision**: Persist one content-free JSON record per operation in app-private
storage. Under an in-process keyed lock, write a sibling temporary file, sync it,
rename it atomically, sync the parent directory, and enforce revision
compare-and-swap.

**Rationale**: The feature has a small bounded record set and no query-heavy data.
Atomic records provide crash recovery and explicit CAS without adding an embedded
database before memo persistence requirements exist. Transcript text remains
outside this store.

**Alternatives considered**: In-memory state cannot survive termination. A single
JSON file increases contention and corruption blast radius. SQLite transactions
are robust but add native build and migration surface disproportionate to this
bounded non-content store; it can replace the repository adapter later without
changing the core.

## Decision 9: Keep credentials ephemeral and redacted

**Decision**: An authorization adapter returns a short-lived opaque token
immediately before dispatch. The token is never serialized or persisted and has
redacted `Debug`/`Display`. Only the HTTP adapter constructs the Bearer header.

**Rationale**: Bearer credentials must be protected in storage and transport;
keeping them outside domain records, IPC, logs, and URLs minimizes exposure. A
401 is user-actionable and stops automatic network work.

**Alternatives considered**: Passing tokens through WebView commands or storing
them in operation records was rejected. Token refresh UI remains outside Issue
#5.

**Primary source**: [RFC 6750](https://www.rfc-editor.org/rfc/rfc6750)

## Decision 10: Separate trusted source access from recorder integration

**Decision**: Define `SourceAudioPort` by opaque source ID and return a validated
descriptor plus an adapter-private stream locator. Implement app-private source
manifest/file validation and deterministic fixture registration, but defer
binding recorder results to that manifest to Issue #6.

**Rationale**: The current public recorder descriptor intentionally hides its
native URI, and the native artifact ID alone cannot reopen the file after
restart. Deriving a path in the core would violate the hexagonal boundary.

**Alternatives considered**: Accepting a path from the WebView or reconstructing
native paths from IDs was rejected for security and correctness.

## Decision 11: Test the core and wire independently

**Decision**: Use fake ports, repository failure injection, fake clocks, and 100
deterministic race/retry orderings for the core. Test the HTTP adapter against an
in-process loopback server and retain the existing OpenAPI Vitest test as the
server-contract oracle. Add content canaries to every observable diagnostic.

**Rationale**: Core tests prove policy without network or platform. Wire tests
prove exact headers, multipart fields, response mapping, timeout, cancellation,
and malformed-response behavior without OpenAI or secrets.

**Alternatives considered**: Mocking only the HTTP adapter would not prove wire
conformance. Network-backed provider tests are nondeterministic and prohibited.

## Decision 12: Require device fixture evidence but no microphone

**Decision**: On physical iPhone and Android, inject the same short,
non-sensitive app-private fixture through a test-only source adapter and a
non-production HTTPS backend. Validate success plus offline or timeout recovery.

**Rationale**: This feature changes mobile network and filesystem behavior but
not recording. Device evidence is required without requesting microphone
permission or duplicating Issue #10 recorder acceptance.

**Alternatives considered**: Simulator-only evidence is insufficient under the
constitution. Re-recording a phrase would add an unrelated microphone dependency.

## Resolved unknowns and deferred ownership

No `NEEDS CLARIFICATION` items remain. Issue #6 owns recorder-source registration,
React/TanStack Query integration, and production journey composition. Issue #7
owns transcript/memo persistence and source-retention choice. Production backend,
provider dispatch, sign-in/token-refresh UX, background transfer, Android native
recording, realtime transcription, and desktop release remain out of scope.
