# Canonical Contract and Generation Contract

## Canonical source

`contracts/transcription-api/v1/openapi.json` is the only authored v1
transcription wire contract. Package metadata, tests, generated manifests, and
future generated wire types reference this path and may not copy it as a second
source of truth.

## Deterministic output

The initial generated artifact is:

`contracts/transcription-api/v1/generated/contract-manifest.json`

It contains only:

- generator format version;
- canonical repository-relative path;
- OpenAPI version and API info version;
- lowercase SHA-256 of exact canonical bytes.

Keys and whitespace are stable. It contains no timestamp, machine path, username,
credential, audio, transcript, or provider configuration.

## Commands

- Generate mode computes expected bytes and updates the tracked artifact.
- Check mode computes expected bytes in memory and fails if the tracked artifact
  is missing or differs.
- Running generate twice with unchanged input produces no git diff.

Future generated types must register their output and generator in this contract,
derive exclusively from the canonical OpenAPI file, and join the same drift check.
