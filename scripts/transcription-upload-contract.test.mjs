import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vitest";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const featureRoot = join(repositoryRoot, "specs/004-transcription-upload-usecase");
const openApiPath = join(repositoryRoot, "contracts/transcription-api/v1/openapi.json");

const featureFile = (relativePath) => join(featureRoot, relativePath);
const readFeatureFile = (relativePath) => readFileSync(featureFile(relativePath), "utf8");
const readOpenApi = () => JSON.parse(readFileSync(openApiPath, "utf8"));

const extractCodeBlockAfter = (document, heading) => {
  const headingOffset = document.indexOf(heading);
  expect(headingOffset, `missing heading: ${heading}`).toBeGreaterThanOrEqual(0);

  const blockStart = document.indexOf("```text", headingOffset);
  const contentStart = blockStart + "```text".length;
  const blockEnd = document.indexOf("```", contentStart);

  expect(blockStart, `missing text block after: ${heading}`).toBeGreaterThanOrEqual(0);
  expect(blockEnd, `unterminated text block after: ${heading}`).toBeGreaterThan(contentStart);

  return document.slice(contentStart, blockEnd);
};

describe("transcription upload contract package", () => {
  test("publishes every required specification artifact", () => {
    const requiredArtifacts = [
      "spec.md",
      "plan.md",
      "tasks.md",
      "research.md",
      "data-model.md",
      "quickstart.md",
      "contracts/transcription-ports.md",
      "contracts/tauri-commands.md",
      "checklists/requirements.md",
    ];

    expect(
      requiredArtifacts.filter((relativePath) => !existsSync(featureFile(relativePath))),
    ).toEqual([]);
  });

  test("keeps the core isolated behind the documented hexagonal paths", () => {
    const plan = readFeatureFile("plan.md");
    const ports = readFeatureFile("contracts/transcription-ports.md");
    const tasks = readFeatureFile("tasks.md");
    const architectureDocuments = `${plan}\n${tasks}`;

    for (const path of [
      "src-tauri/crates/transcription-core/src/domain.rs",
      "src-tauri/crates/transcription-core/src/ports.rs",
      "src-tauri/crates/transcription-core/src/application.rs",
      "src-tauri/src/inbound/transcription_commands.rs",
      "src-tauri/src/infrastructure/transcription/http_backend.rs",
      "src-tauri/tests/transcription_http_contract.rs",
    ]) {
      expect(architectureDocuments).toContain(path);
    }

    expect(ports).toContain(
      "Ports never expose Tauri, HTTP, filesystem paths, provider types, or\ncredentials",
    );
    expect(ports).toContain("It alone owns state transitions");
    expect(plan).toContain("Do not create a native plugin");
    expect(plan).toContain("Issue #5 adds no React state");
  });

  test("defines the backend port against the versioned API contract", () => {
    const openApi = readOpenApi();
    const ports = readFeatureFile("contracts/transcription-ports.md");
    const research = readFeatureFile("research.md");

    expect(openApi.info.version).toBe("1.0.0");
    expect(openApi.security).toEqual([{ bearerAuth: [] }]);
    expect(openApi.paths["/v1/transcriptions"].post).toBeDefined();
    expect(openApi.paths["/v1/transcriptions/{operationId}"].get).toBeDefined();
    expect(openApi.paths["/v1/transcriptions/{operationId}"].delete).toBeDefined();

    for (const operation of ["`create`", "`get`", "`delete`"]) {
      expect(ports).toContain(operation);
    }

    expect(ports).toContain("source/options fingerprint and idempotency key");
    expect(research).toContain("exact multipart request with the same idempotency key");
    expect(research).toContain("When the backend ID is known, uncertain outcomes resolve by GET");
  });

  test("keeps sensitive and adapter-owned fields outside public DTO shapes", () => {
    const commands = readFeatureFile("contracts/tauri-commands.md");
    const dataModel = readFeatureFile("data-model.md");
    const operationView = extractCodeBlockAfter(commands, "## OperationView");
    const errorDto = extractCodeBlockAfter(commands, "## Error DTO");
    const eventDto = extractCodeBlockAfter(commands, "## Advisory event");
    const publicShapes = `${operationView}\n${errorDto}\n${eventDto}`.toLowerCase();

    for (const forbiddenField of [
      "backendurl",
      "filesystempath",
      "authorizationtoken",
      "idempotencykey",
      "rawchecksum",
      "providermodel",
      "providerpayload",
      "signedurl",
      "storagepath",
      "audiobytes",
    ]) {
      expect(publicShapes).not.toContain(forbiddenField);
    }

    expect(commands).toContain(
      "The WebView cannot provide a backend URL, filesystem path, authorization token",
    );
    expect(commands).toContain("Transcript text is never emitted in an event");
    expect(dataModel).toContain(
      "Exclude transcript text,\nsource URI/path, audio bytes, token, authorization header, provider body, signed\nURL, and raw error",
    );
  });

  test("publishes only the five deliberate command operations", () => {
    const commands = readFeatureFile("contracts/tauri-commands.md");
    const commandNames = [...commands.matchAll(/\| `(transcription_[a-z]+)`\s+\|/g)].map(
      ([, name]) => name,
    );

    expect(commandNames).toEqual([
      "transcription_submit",
      "transcription_status",
      "transcription_retry",
      "transcription_cancel",
      "transcription_recover",
    ]);
    expect(commands).toContain("Event name: `transcription://event`");
    expect(commands).toContain("Do not add\ngeneric HTTP or filesystem capability");
  });

  test("uses contiguous actionable task IDs and preserves physical gates", () => {
    const tasks = readFeatureFile("tasks.md");
    const taskLines = tasks.split("\n").filter((line) => line.startsWith("- ["));
    const taskPattern = /^- \[[ xX]] T\d{3}(?: \[P])?(?: \[US[1-3]])? .+ `[^`]+`.*$/;

    expect(taskLines.length).toBeGreaterThan(0);
    for (const line of taskLines) expect(line, line).toMatch(taskPattern);

    const taskIds = taskLines.map((line) => Number(line.match(/T(\d{3})/)[1]));
    expect(taskIds).toEqual(Array.from({ length: taskIds.length }, (_, index) => index + 1));

    for (const physicalTask of ["T036", "T037"]) {
      expect(tasks).toMatch(new RegExp(`^- \\[ \\] ${physicalTask} .+physical`, "m"));
    }

    expect(tasks).toContain(
      "Do not mark T036 or T037 complete without actual physical-device evidence",
    );
  });
});
