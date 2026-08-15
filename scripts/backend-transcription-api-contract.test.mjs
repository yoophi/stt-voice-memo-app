import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vitest";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const contractPath = join(repositoryRoot, "contracts/transcription-api/v1/openapi.json");

const readContract = () => JSON.parse(readFileSync(contractPath, "utf8"));

const resolveLocalReference = (document, reference) => {
  expect(reference.startsWith("#/"), `external reference: ${reference}`).toBe(true);

  return reference
    .slice(2)
    .split("/")
    .map((segment) => segment.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, segment) => value?.[segment], document);
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
    const problem = contract.components.schemas.Problem;
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
    }

    const createResponses = contract.paths["/v1/transcriptions"].post.responses;
    expect(Object.keys(createResponses)).toEqual(
      expect.arrayContaining(["400", "401", "403", "413", "415", "422", "429"]),
    );
    expect(contract["x-contract-policy"].ownership.non_owner_status).toBe(404);
    expect(contract["x-contract-policy"].ownership.unknown_status).toBe(404);
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
