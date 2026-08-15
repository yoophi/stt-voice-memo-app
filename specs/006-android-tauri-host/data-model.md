# Data Model: Android Host Validation

This feature has no product-domain persistence. These models define repository
validation and evidence only.

## AndroidHostBaseline

| Field              | Type                     | Invariant                           |
| ------------------ | ------------------------ | ----------------------------------- |
| projectPath        | repository-relative path | exactly `src-tauri/gen/android`     |
| applicationId      | string                   | exactly `com.yoophi.sttvoicememo`   |
| debugApplicationId | string                   | application ID with `.debug` suffix |
| activity           | component name           | exactly `.MainActivity`             |
| minSdk             | integer                  | exactly `24`                        |
| frontendDist       | repository-relative path | exactly `../dist` in Tauri config   |
| requiredFiles      | set of paths             | every member exists and is tracked  |
| generatedBy        | CLI baseline             | lockfile-pinned Tauri CLI           |

## CapabilityAllowlist

| Category                | Allowed values                                 |
| ----------------------- | ---------------------------------------------- |
| permissions             | empty set                                      |
| features                | required `android.hardware.touchscreen` only   |
| activities              | `.MainActivity` only                           |
| exported components     | `.MainActivity` only                           |
| activity intent filters | one `MAIN` action plus one `LAUNCHER` category |
| providers               | empty set                                      |
| services                | empty set                                      |
| receivers               | empty set                                      |
| activity aliases        | empty set                                      |

Any unknown app-owned member is invalid. The packaged/merged manifest keeps a
second exact allowlist for AndroidX runtime components and the application-scoped,
signature-protected dynamic-receiver permission; there is no permissive fallback
and no Android system permission is allowed.

## HostValidation

```text
unavailable -> partial -> invalid | verified
```

| State       | Meaning                                                        | Exit status              |
| ----------- | -------------------------------------------------------------- | ------------------------ |
| unavailable | no Android host file exists                                    | non-zero after Issue #24 |
| partial     | at least one host file exists and a required file is absent    | non-zero                 |
| invalid     | complete required file set exists but a contract fails         | non-zero                 |
| verified    | source host, identity, SDK floor, activity, and allowlist pass | zero                     |

Transitions occur only because repository files/configuration change. Validation
never mutates the host and never prints secret values.

## BuildEvidence

| Field          | Constraint                                       |
| -------------- | ------------------------------------------------ |
| revision       | exact Git commit or explicit working-tree marker |
| classification | `automated` or `physical`                        |
| command        | repository-root command without credentials      |
| toolchain      | version strings only                             |
| artifact       | repository-relative build-output path            |
| sha256         | lowercase artifact digest, if produced           |
| result         | `pass`, `fail`, or `not-run`                     |
| failureCode    | stable content-safe category only                |

Issue #24 may write only `automated` evidence. Physical rows remain `not-run` and
are completed by Issue #23.
