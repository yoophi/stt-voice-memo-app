# Configuration and Secret Boundary Contract

## Classes

### Client-safe

- Public build-time values must use the explicitly documented mobile namespace.
- Values are non-secret origins or feature settings that are safe to inspect in
  a bundled application.
- No OpenAI key, backend signing secret, service credential, private storage
  location, or user token is client-safe.

### Backend-only

- Names are declared in `apps/backend/.env.example` with empty or synthetic
  placeholders and safe descriptions.
- Working values live only in ignored local/CI secret stores.
- Backend-only names and values are forbidden in mobile source, `dist/`, source
  maps, generated native resources, examples, logs, and test evidence.

## Required checks

1. Parse backend-only names from the template; do not maintain a second hand
   list in the scanner.
2. Scan text-readable mobile source/config and built artifacts.
3. Accept optional synthetic canary values from the validation process without
   printing those values on failure.
4. Pass a unique synthetic canary through an actual client build, require the
   scanner to reject its transformed output, and never treat a fixture-only scan
   as sufficient build evidence.
5. Report only file path and stable forbidden-name/canary category.
6. Never scan or print arbitrary developer environment values.
7. Delete temporary canary fixtures and build output after tests and ignore local
   environment files.

The template may define future names for provider credentials, backend signing,
storage, persistence, queue, or deployment, but this issue does not select or
implement those providers.
