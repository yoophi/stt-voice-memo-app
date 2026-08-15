# Data Model: Backend Monorepo Workspace

This feature models repository ownership and validation metadata. It introduces
no product database or runtime persistence.

## WorkspaceArea

Represents one owned repository scope.

| Field            | Rules                                                                      |
| ---------------- | -------------------------------------------------------------------------- |
| `id`             | One of `mobile`, `backend`, `contract`, `shared`                           |
| `roots`          | Non-empty, repository-relative paths; no root belongs to two runtime areas |
| `owner`          | Named responsibility, not a person-specific credential                     |
| `consumes`       | May point to `contract` or lower-level modules only                        |
| `commands`       | Development/build/test/lint/format availability for the area               |
| `validationJobs` | CI jobs required when the area changes                                     |

### Invariants

- Mobile and backend runtime areas never consume each other.
- `shared` contains tooling/configuration, not shared mobile/backend business logic.
- Root manifests and validation scripts affect all areas.

## CanonicalContract

Represents the only authored transcription wire source.

| Field             | Rules                                                            |
| ----------------- | ---------------------------------------------------------------- |
| `id`              | `transcription-api-v1`                                           |
| `sourcePath`      | Exactly `contracts/transcription-api/v1/openapi.json`            |
| `contractVersion` | Read from the OpenAPI document metadata                          |
| `sha256`          | Lowercase 64-character digest of exact source bytes              |
| `consumers`       | Mobile adapter, future backend HTTP adapter, contract validation |

### Relationship

`CanonicalContract` has zero or more `DerivedArtifact` records. It is never a
derived artifact of another repository file.

## DerivedArtifact

| Field           | Rules                                                               |
| --------------- | ------------------------------------------------------------------- |
| `path`          | Owned under the canonical contract or a documented consumer package |
| `generator`     | Repository-relative deterministic command                           |
| `sourceSha256`  | Must equal the current canonical contract digest                    |
| `formatVersion` | Stable generator output format version                              |
| `tracked`       | Explicit boolean documented by the owner                            |

### States

```text
missing ──generate──> current
current ──source/manual change──> stale
stale ──generate──> current
```

Validation accepts only `current`. Generation from identical inputs must not
change tracked bytes.

## ConfigurationClass

| Field              | Rules                                                         |
| ------------------ | ------------------------------------------------------------- |
| `id`               | `client-safe` or `backend-only`                               |
| `templatePath`     | Tracked name-only template                                    |
| `runtimeFiles`     | Ignored local paths                                           |
| `allowedConsumers` | Mobile for client-safe; backend only for backend-only         |
| `secretNames`      | Parsed from backend template and forbidden from mobile output |

No configuration entity stores a working secret in repository metadata.

## ValidationScope

| Field         | Rules                                       |
| ------------- | ------------------------------------------- |
| `mobile`      | Boolean                                     |
| `backend`     | Boolean                                     |
| `contract`    | Boolean                                     |
| `reasonCodes` | Sorted, content-free classification reasons |

### Classification Rules

1. A path under one owned root selects that scope.
2. Canonical-contract changes select `contract`, `mobile`, and `backend`.
3. Root manifests, lockfiles, workflow logic, shared scripts, or unknown paths
   select all scopes (fail-safe).
4. Documentation-only changes select the scope they document; feature specs and
   root governance select all because they may redefine dependencies.
5. Empty change lists select all scopes for manual/full runs.

## ValidationEvidence

| Field         | Rules                                                       |
| ------------- | ----------------------------------------------------------- |
| `commit`      | Exact tested revision                                       |
| `environment` | OS/tool or physical device model/version                    |
| `scope`       | One `ValidationScope` or physical platform                  |
| `command`     | Documented root command                                     |
| `outcome`     | `pass`, `fail`, or `not-run`                                |
| `notes`       | No secret, audio, transcript, or private filesystem content |

Physical device `not-run` evidence cannot satisfy the corresponding success
criterion.
