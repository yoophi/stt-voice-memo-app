import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vitest";

import { createTranscriptionContractDouble } from "./support/backend-transcription-contract-double.mjs";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const contractPath = join(repositoryRoot, "contracts/transcription-api/v1/openapi.json");

const readContract = () => JSON.parse(readFileSync(contractPath, "utf8"));

const createContractHarness = ({
  contract = readContract(),
  transcribe = () => ({ text: "Synthetic result" }),
  remove = () => "pending",
} = {}) => {
  const provider = {
    dispatchCount: 0,
    async transcribe(request) {
      this.dispatchCount += 1;
      return transcribe({ dispatchCount: this.dispatchCount, request });
    },
  };
  const cleanup = {
    attemptCount: 0,
    async remove(request) {
      this.attemptCount += 1;
      return remove({ attemptCount: this.attemptCount, request });
    },
  };

  return {
    contract,
    cleanup,
    provider,
    boundary: createTranscriptionContractDouble({
      cleanup,
      provider,
      policy: contract["x-contract-policy"],
    }),
  };
};

const resolveLocalReference = (document, reference) => {
  expect(reference.startsWith("#/"), `external reference: ${reference}`).toBe(true);

  return reference
    .slice(2)
    .split("/")
    .map((segment) => segment.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, segment) => value?.[segment], document);
};

const dereference = (document, value) => {
  let resolved = value;
  while (resolved?.$ref) resolved = resolveLocalReference(document, resolved.$ref);
  return resolved;
};

const collectReferences = (value, references = []) => {
  if (Array.isArray(value)) {
    for (const item of value) collectReferences(item, references);
  } else if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      if (key === "$ref") references.push(child);
      else collectReferences(child, references);
    }
  }

  return references;
};

describe("backend transcription API contract", () => {
  test("concurrent matching submissions dispatch one provider operation", async () => {
    const { boundary, provider } = createContractHarness();
    const submission = {
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-recording-0001",
      fingerprint: "sha256:recording-0001",
      sourceAudioId: "audio-1",
    };

    const responses = await Promise.all(
      Array.from({ length: 100 }, () => boundary.submit(submission)),
    );
    const completed = await boundary.read({
      bearerPrincipal: submission.bearerPrincipal,
      operationId: responses[0].body.id,
    });

    expect(new Set(responses.map((response) => response.body.id)).size).toBe(1);
    expect(responses.every((response) => response.status === 202)).toBe(true);
    expect(responses.every((response) => response.body.result === undefined)).toBe(true);
    expect(completed.body).toMatchObject({
      state: "completed",
      result: { text: "Synthetic result" },
    });
    expect(provider.dispatchCount).toBe(1);
  });

  test("changed content under the same key is rejected before provider dispatch", async () => {
    const { boundary, provider } = createContractHarness();
    const original = {
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-recording-0001",
      fingerprint: "sha256:recording-0001",
      sourceAudioId: "audio-1",
    };

    await boundary.submit(original);
    const conflict = await boundary.submit({
      ...original,
      fingerprint: "sha256:different-recording",
    });

    expect(conflict).toMatchObject({
      status: 422,
      body: { code: "IDEMPOTENCY_MISMATCH", category: "terminal", retryable: false },
    });
    expect(provider.dispatchCount).toBe(1);
  });

  test("missing authentication is rejected before provider dispatch", async () => {
    const { boundary, provider } = createContractHarness();

    const response = await boundary.submit({
      bearerPrincipal: null,
      idempotencyKey: "idem-key-for-recording-0001",
      fingerprint: "sha256:recording-0001",
      sourceAudioId: "audio-1",
    });

    expect(response).toMatchObject({
      status: 401,
      body: { code: "AUTHENTICATION_REQUIRED", category: "user_actionable", retryable: false },
    });
    expect(provider.dispatchCount).toBe(0);
  });

  test("daily usage rejection happens before provider dispatch", async () => {
    const { boundary, provider } = createContractHarness();

    const response = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-recording-0001",
      fingerprint: "sha256:recording-0001",
      sourceAudioId: "audio-1",
      dailyUsageAllowed: false,
    });

    expect(response).toMatchObject({
      status: 429,
      body: { code: "USAGE_LIMIT_EXCEEDED", category: "user_actionable", retryable: false },
    });
    expect(provider.dispatchCount).toBe(0);
  });

  test("rolling create limit rejects excess work before provider dispatch", async () => {
    const contract = readContract();
    const { boundary, provider } = createContractHarness({ contract });
    const limit = contract["x-contract-policy"].limits.create_per_rolling_minute;

    for (let index = 0; index < limit; index += 1) {
      const accepted = await boundary.submit({
        bearerPrincipal: "user-1",
        idempotencyKey: `idem-key-for-recording-${String(index).padStart(4, "0")}`,
        fingerprint: `sha256:recording-${index}`,
        sourceAudioId: `audio-${index}`,
      });
      expect(accepted.status).toBe(202);
    }

    const limited = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-recording-over-limit",
      fingerprint: "sha256:over-limit",
      sourceAudioId: "audio-over-limit",
    });

    expect(limited).toMatchObject({
      status: 429,
      body: { code: "RATE_LIMITED", category: "retryable", retryable: true },
    });
    expect(provider.dispatchCount).toBe(limit);
  });

  test("unknown and cross-owner reads are indistinguishable", async () => {
    const { boundary } = createContractHarness();
    const accepted = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-recording-0001",
      fingerprint: "sha256:recording-0001",
      sourceAudioId: "audio-1",
    });

    const crossOwner = await boundary.read({
      bearerPrincipal: "user-2",
      operationId: accepted.body.id,
    });
    const unknown = await boundary.read({
      bearerPrincipal: "user-2",
      operationId: "unknown-operation",
    });

    expect(crossOwner).toEqual(unknown);
    expect(unknown).toMatchObject({
      status: 404,
      body: { code: "OPERATION_NOT_FOUND", category: "terminal", retryable: false },
    });
  });

  test("active operation limit rejects excess work before provider dispatch", async () => {
    const contract = readContract();
    const pendingResolvers = [];
    const { boundary, provider } = createContractHarness({
      contract,
      transcribe() {
        return new Promise((resolve) => pendingResolvers.push(resolve));
      },
    });
    const activeLimit = contract["x-contract-policy"].limits.active_operations_per_user;
    const active = Array.from({ length: activeLimit }, (_, index) =>
      boundary.submit({
        bearerPrincipal: "user-1",
        idempotencyKey: `idem-key-for-active-recording-${index}`,
        fingerprint: `sha256:active-recording-${index}`,
        sourceAudioId: `active-audio-${index}`,
      }),
    );

    const limited = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-active-recording-over-limit",
      fingerprint: "sha256:active-over-limit",
      sourceAudioId: "active-audio-over-limit",
    });

    expect(limited).toMatchObject({
      status: 429,
      body: { code: "RATE_LIMITED", category: "retryable", retryable: true },
    });
    expect(provider.dispatchCount).toBe(activeLimit);

    const accepted = await Promise.all(active);
    pendingResolvers[0]({ text: "Synthetic result" });
    await boundary.read({
      bearerPrincipal: "user-1",
      operationId: accepted[0].body.id,
    });

    const reopenedSlot = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-reopened-slot",
      fingerprint: "sha256:reopened-slot",
      sourceAudioId: "audio-reopened-slot",
    });
    const stillLimited = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-still-limited",
      fingerprint: "sha256:still-limited",
      sourceAudioId: "audio-still-limited",
    });

    expect(reopenedSlot.status).toBe(202);
    expect(stillLimited.status).toBe(429);
    expect(provider.dispatchCount).toBe(activeLimit + 1);

    for (const resolve of pendingResolvers.slice(1)) resolve({ text: "Synthetic result" });
  });

  test("deleting active work hides and discards a late provider result", async () => {
    let finishProvider;
    const { boundary, cleanup } = createContractHarness({
      transcribe: () => new Promise((resolve) => (finishProvider = resolve)),
    });
    const accepted = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-delete-0001",
      fingerprint: "sha256:delete-0001",
      sourceAudioId: "audio-delete-1",
    });

    const deleting = await boundary.delete({
      bearerPrincipal: "user-1",
      operationId: accepted.body.id,
    });
    finishProvider({ text: "Must be discarded" });
    const afterLateResult = await boundary.read({
      bearerPrincipal: "user-1",
      operationId: accepted.body.id,
    });

    expect(deleting).toMatchObject({
      status: 202,
      body: {
        state: "deleting",
        cleanup: { state: "in_progress", content_available: false },
      },
    });
    expect(afterLateResult.body.state).toBe("deleting");
    expect(afterLateResult.body.result).toBeUndefined();
    expect(cleanup.attemptCount).toBe(1);
  });

  test("repeated delete retries cleanup and becomes idempotent after completion", async () => {
    const { boundary, cleanup } = createContractHarness({
      remove: ({ attemptCount }) => (attemptCount === 1 ? "failed" : "completed"),
    });
    const accepted = await boundary.submit({
      bearerPrincipal: "user-1",
      idempotencyKey: "idem-key-for-cleanup-retry",
      fingerprint: "sha256:cleanup-retry",
      sourceAudioId: "audio-cleanup-retry",
    });

    const failedCleanup = await boundary.delete({
      bearerPrincipal: "user-1",
      operationId: accepted.body.id,
    });
    const completedCleanup = await boundary.delete({
      bearerPrincipal: "user-1",
      operationId: accepted.body.id,
    });
    const idempotentReplay = await boundary.delete({
      bearerPrincipal: "user-1",
      operationId: accepted.body.id,
    });

    expect(failedCleanup).toMatchObject({
      status: 202,
      body: { state: "deleting", cleanup: { state: "failed_retrying" } },
    });
    expect(completedCleanup.status).toBe(204);
    expect(idempotentReplay.status).toBe(204);
    expect(cleanup.attemptCount).toBe(2);
  });

  test("publishes a parseable OpenAPI 3.1.1 artifact with local references", () => {
    expect(existsSync(contractPath)).toBe(true);

    const contract = readContract();
    expect(contract.openapi).toBe("3.1.1");
    expect(contract.info.version).toBe("1.0.0");

    for (const reference of collectReferences(contract)) {
      expect(resolveLocalReference(contract, reference), reference).toBeDefined();
    }
  });

  test("defines authenticated asynchronous multipart submission", () => {
    const contract = readContract();
    const create = contract.paths["/v1/transcriptions"].post;
    const multipart = create.requestBody.content["multipart/form-data"].schema;
    const parameterNames = create.parameters.map((parameter) => parameter.name);

    expect(contract.security).toEqual([{ bearerAuth: [] }]);
    expect(contract.components.securitySchemes.bearerAuth).toMatchObject({
      type: "http",
      scheme: "bearer",
    });
    expect(parameterNames).toEqual(expect.arrayContaining(["Idempotency-Key", "X-Audio-SHA256"]));
    expect(multipart.required).toEqual(["audio", "source_audio_id"]);
    expect(multipart.properties.audio).toMatchObject({
      type: "string",
      format: "binary",
    });
    expect(multipart.properties.language_hint).toBeDefined();
    expect(multipart.properties.model).toBeUndefined();

    const accepted = create.responses["202"];
    expect(Object.keys(accepted.headers)).toEqual(
      expect.arrayContaining(["Location", "Retry-After", "Cache-Control", "X-Request-Id"]),
    );
    expect(accepted.content["application/json"].schema.$ref).toBe(
      "#/components/schemas/TranscriptionOperation",
    );
  });

  test("returns provider-neutral status and final result", () => {
    const contract = readContract();
    const read = contract.paths["/v1/transcriptions/{operationId}"].get;
    const states = contract.components.schemas.OperationState.enum;
    const queued = contract.components.examples.QueuedOperation.value;
    const completed = contract.components.examples.CompletedOperation.value;
    const failed = contract.components.examples.FailedOperation.value;

    expect(read.responses["200"].content["application/json"].schema.$ref).toBe(
      "#/components/schemas/TranscriptionOperation",
    );
    expect(states).toEqual(expect.arrayContaining(["queued", "processing", "completed"]));
    expect(queued.result).toBeUndefined();
    expect(completed).toMatchObject({
      state: "completed",
      result: { text: "Example memo text." },
    });
    expect(contract.components.schemas.TranscriptionOperation.required).toContain("request_id");
    expect(failed).toMatchObject({
      state: "failed",
      failure: { code: "PROCESSING_TIMEOUT", category: "uncertain", retryable: false },
    });

    const serializedContract = JSON.stringify(contract);
    expect(serializedContract).not.toContain("openai_api_key");
    expect(serializedContract).not.toContain("provider_model");
  });

  test("defines replay, conflict, and uncertain retry semantics", () => {
    const contract = readContract();
    const create = contract.paths["/v1/transcriptions"].post;
    const policy = contract["x-contract-policy"].idempotency;

    expect(Object.keys(create.responses)).toEqual(
      expect.arrayContaining(["200", "202", "409", "422", "429", "500", "503", "504"]),
    );
    expect(create.responses["200"].headers["Idempotency-Replayed"]).toBeDefined();
    expect(policy).toMatchObject({
      scope: "authenticated_owner_and_create_endpoint",
      fingerprint: [
        "verified_audio_sha256",
        "source_audio_id",
        "normalized_language_hint",
        "contract_version",
      ],
      same_key_same_fingerprint: "return_existing_operation",
      same_key_different_fingerprint_status: 422,
      concurrent_reservation_conflict_status: 409,
      tombstone_days: 7,
    });
  });

  test("uses typed problem details for every recovery error", () => {
    const contract = readContract();
    const problem = contract.components.schemas.ProblemBase;
    const categories = contract.components.schemas.ProblemCategory.enum;
    const expected = {
      OperationConflict: [409, "OPERATION_CONFLICT", "uncertain", false],
      ContentExpired: [410, "CONTENT_EXPIRED", "terminal", false],
      IdempotencyMismatch: [422, "IDEMPOTENCY_MISMATCH", "terminal", false],
      RateLimited: [429, "RATE_LIMITED", "retryable", true],
      InternalError: [500, "INTERNAL_ERROR", "uncertain", false],
      ProviderUnavailable: [503, "PROVIDER_UNAVAILABLE", "retryable", true],
      ProcessingTimeout: [504, "PROCESSING_TIMEOUT", "uncertain", false],
    };

    expect(problem.required).toEqual(
      expect.arrayContaining([
        "type",
        "title",
        "status",
        "code",
        "category",
        "retryable",
        "request_id",
      ]),
    );
    expect(categories).toEqual(["retryable", "user_actionable", "terminal", "uncertain"]);

    for (const [name, [status, code, category, retryable]] of Object.entries(expected)) {
      expect(contract.components.examples[name].value).toMatchObject({
        status,
        code,
        category,
        retryable,
      });
    }
  });

  test("couples every HTTP error response to the same problem status", () => {
    const contract = readContract();

    for (const path of Object.values(contract.paths)) {
      for (const operation of [path.post, path.get, path.delete].filter(Boolean)) {
        for (const [status, declaredResponse] of Object.entries(operation.responses)) {
          if (Number(status) < 400) continue;

          const response = dereference(contract, declaredResponse);
          const schemaReference = response.content["application/problem+json"].schema.$ref;
          expect(schemaReference).toBe(`#/components/schemas/Problem${status}`);

          const statusSchema = resolveLocalReference(contract, schemaReference);
          expect(statusSchema.allOf).toEqual(
            expect.arrayContaining([
              {
                type: "object",
                properties: { status: { const: Number(status) } },
                required: ["status"],
              },
            ]),
          );
        }
      }
    }
  });

  test("defines idempotent cancellation, cleanup, and expiry", () => {
    const contract = readContract();
    const operationPath = contract.paths["/v1/transcriptions/{operationId}"];
    const remove = operationPath.delete;
    const cancellation = contract["x-contract-policy"].cancellation;

    expect(Object.keys(remove.responses)).toEqual(
      expect.arrayContaining(["202", "204", "404", "410"]),
    );
    expect(remove.requestBody).toBeUndefined();
    expect(remove.responses["202"].content["application/json"].schema.$ref).toBe(
      "#/components/schemas/TranscriptionOperation",
    );
    expect(remove.responses["204"].content).toBeUndefined();
    expect(remove.responses["204"].headers["X-Request-Id"]).toBeDefined();
    expect(cancellation).toMatchObject({
      invalidates_queued_or_processing_work: true,
      late_provider_result: "discard",
      content_available_after_delete: false,
    });

    for (const name of [
      "FailedOperation",
      "CancelledOperation",
      "DeletingOperation",
      "DeletedOperation",
    ]) {
      expect(contract.components.examples[name]).toBeDefined();
    }
    expect(contract.components.examples.CleanupRetryingOperation.value).toMatchObject({
      state: "deleting",
      cleanup: {
        state: "failed_retrying",
        content_available: false,
        delete_by: "2026-08-16T02:00:08Z",
      },
    });
    expect(contract.components.examples.CancelledOperation.value.cleanup.delete_by).toBe(
      "2026-08-16T02:00:05Z",
    );
    expect(contract.components.examples.DeletingOperation.value.cleanup.delete_by).toBe(
      "2026-08-16T02:00:06Z",
    );
    expect(contract.components.schemas.CleanupStatus.allOf).toEqual(
      expect.arrayContaining([
        {
          if: {
            properties: {
              state: { enum: ["scheduled", "in_progress", "failed_retrying"] },
            },
            required: ["state"],
          },
          then: { required: ["delete_by"] },
        },
      ]),
    );
  });

  test("enumerates every validation, ownership, and usage error", () => {
    const contract = readContract();
    const expected = {
      MalformedRequest: [400, "MALFORMED_REQUEST", "terminal", false],
      AuthenticationRequired: [401, "AUTHENTICATION_REQUIRED", "user_actionable", false],
      FeatureNotAllowed: [403, "FEATURE_NOT_ALLOWED", "user_actionable", false],
      OperationNotFound: [404, "OPERATION_NOT_FOUND", "terminal", false],
      OperationConflict: [409, "OPERATION_CONFLICT", "uncertain", false],
      ContentExpired: [410, "CONTENT_EXPIRED", "terminal", false],
      AudioTooLarge: [413, "AUDIO_TOO_LARGE", "user_actionable", false],
      UnsupportedAudio: [415, "UNSUPPORTED_AUDIO", "user_actionable", false],
      AudioDurationExceeded: [422, "AUDIO_DURATION_EXCEEDED", "user_actionable", false],
      InvalidLanguageHint: [422, "INVALID_LANGUAGE_HINT", "user_actionable", false],
      ChecksumMismatch: [422, "CHECKSUM_MISMATCH", "terminal", false],
      IdempotencyMismatch: [422, "IDEMPOTENCY_MISMATCH", "terminal", false],
      RateLimited: [429, "RATE_LIMITED", "retryable", true],
      UsageLimitExceeded: [429, "USAGE_LIMIT_EXCEEDED", "user_actionable", false],
      InternalError: [500, "INTERNAL_ERROR", "uncertain", false],
      ProviderUnavailable: [503, "PROVIDER_UNAVAILABLE", "retryable", true],
      ProcessingTimeout: [504, "PROCESSING_TIMEOUT", "uncertain", false],
    };

    for (const [name, [status, code, category, retryable]] of Object.entries(expected)) {
      expect(contract.components.examples[name]?.value, name).toMatchObject({
        status,
        code,
        category,
        retryable,
      });

      const variant = contract.components.schemas.FailureTuple.oneOf.find(
        (candidate) => candidate.properties.code.const === code,
      );
      expect(variant.properties).toMatchObject({
        status: { const: status },
        code: { const: code },
        category: { const: category },
        retryable: { const: retryable },
      });
    }

    expect(contract.components.schemas.FailureTuple.oneOf).toHaveLength(
      Object.keys(expected).length,
    );
    expect(contract.components.schemas.Problem.allOf).toEqual(
      expect.arrayContaining([{ $ref: "#/components/schemas/FailureTuple" }]),
    );
    expect(contract.components.schemas.OperationFailure.allOf).toEqual([
      { $ref: "#/components/schemas/FailureTuple" },
    ]);

    const createResponses = contract.paths["/v1/transcriptions"].post.responses;
    expect(Object.keys(createResponses)).toEqual(
      expect.arrayContaining(["400", "401", "403", "413", "415", "422", "429"]),
    );
    expect(contract["x-contract-policy"].ownership.non_owner_status).toBe(404);
    expect(contract["x-contract-policy"].ownership.unknown_status).toBe(404);

    const semanticExamples = createResponses["422"].content["application/problem+json"].examples;
    expect(Object.keys(semanticExamples)).toEqual(
      expect.arrayContaining([
        "audioDurationExceeded",
        "invalidLanguageHint",
        "checksumMismatch",
        "idempotencyMismatch",
      ]),
    );
    const limitExamples = createResponses["429"].content["application/problem+json"].examples;
    expect(Object.keys(limitExamples)).toEqual(
      expect.arrayContaining(["rateLimited", "usageLimitExceeded"]),
    );
  });

  test("makes limits, retention, logging, and provider isolation executable", () => {
    const policy = readContract()["x-contract-policy"];

    expect(policy.limits).toMatchObject({
      create_per_rolling_minute: 10,
      active_operations_per_user: 3,
      management_per_rolling_minute: 60,
      daily_usage_pre_dispatch: true,
      max_audio_bytes: 25000000,
      max_duration_seconds: 600,
      upload_timeout_seconds: 120,
      terminal_timeout_seconds: 600,
    });
    expect(policy.retention).toMatchObject({
      terminal_content_delete_hours: 24,
      idempotency_tombstone_days: 7,
      rejected_upload_delete: "immediately",
    });
    expect(policy.logging.excluded_fields).toEqual(
      expect.arrayContaining([
        "audio",
        "transcript",
        "authorization",
        "credential",
        "provider_response",
        "storage_path",
      ]),
    );
    expect(policy.provider).toMatchObject({
      client_model_selection: false,
      credentials_in_client: false,
      calls_in_contract_tests: false,
      accepted_audio_formats: ["mp3", "mp4", "mpeg", "mpga", "m4a", "wav", "webm"],
    });
    expect(policy.input_validation).toEqual({
      audio_parts: "exactly_one",
      integrity_source: "server_verified_decoded_bytes",
      declared_media_type_is_sufficient: false,
    });
  });

  test("exposes only the versioned provider-neutral local contract", () => {
    const contract = readContract();
    const serialized = JSON.stringify(contract).toLowerCase();
    const multipart =
      contract.paths["/v1/transcriptions"].post.requestBody.content["multipart/form-data"].schema;

    expect(Object.keys(contract.paths)).toEqual([
      "/v1/transcriptions",
      "/v1/transcriptions/{operationId}",
    ]);
    expect(contract.servers).toEqual([{ url: "/" }]);
    expect(Object.keys(multipart.properties)).toEqual([
      "audio",
      "source_audio_id",
      "language_hint",
    ]);
    expect(collectReferences(contract).every((reference) => reference.startsWith("#/"))).toBe(true);
    for (const forbidden of [
      "sk-",
      "api.openai.com",
      "openai_api_key",
      "provider_model",
      "signed_url",
      "stack_trace",
    ]) {
      expect(serialized, forbidden).not.toContain(forbidden);
    }
  });
});
