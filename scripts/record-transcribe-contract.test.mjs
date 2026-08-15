import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, test } from "vitest";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const featureRoot = join(repositoryRoot, "specs/002-record-transcribe-journey");

const featureFile = (relativePath) => join(featureRoot, relativePath);
const readFeatureFile = (relativePath) => readFileSync(featureFile(relativePath), "utf8");

describe("record-and-transcribe contract package", () => {
  test("publishes every required artifact", () => {
    const requiredArtifacts = [
      "spec.md",
      "plan.md",
      "tasks.md",
      "research.md",
      "data-model.md",
      "quickstart.md",
      "contracts/journey-state-machine.md",
      "contracts/recorder-port.md",
      "contracts/transcription-boundary.md",
      "checklists/requirements.md",
      "checklists/implementation-readiness.md",
    ];

    expect(
      requiredArtifacts.filter((relativePath) => !existsSync(featureFile(relativePath))),
    ).toEqual([]);
  });

  test("traces the primary recording-to-memo journey", () => {
    const readiness = readFeatureFile("checklists/implementation-readiness.md");
    const stateMachine = readFeatureFile("contracts/journey-state-machine.md");
    const dataModel = readFeatureFile("data-model.md");

    for (const evidenceId of [
      "US1-RECORD",
      "US1-FINAL",
      "US1-EDIT",
      "US1-SAVE",
      "US1-DELETE-DEFAULT",
    ]) {
      expect(readiness).toContain(evidenceId);
    }

    for (const state of [
      "recording",
      "finalizing",
      "ready",
      "uploading",
      "transcribing",
      "editable_draft",
      "saving",
      "saved",
    ]) {
      expect(stateMachine).toContain(`\`${state}\``);
    }

    for (const identity of [
      "RecordingSessionId",
      "SourceAudioId",
      "TranscriptionOperationId",
      "MemoId",
    ]) {
      expect(dataModel).toContain(identity);
    }
  });

  test("defines every required recovery outcome", () => {
    const readiness = readFeatureFile("checklists/implementation-readiness.md");
    const stateMachine = readFeatureFile("contracts/journey-state-machine.md");
    const recorder = readFeatureFile("contracts/recorder-port.md");

    for (const evidenceId of [
      "REC-PERMISSION",
      "REC-INTERRUPTION",
      "REC-BACKGROUND",
      "REC-OFFLINE",
      "REC-UNCERTAIN",
      "REC-DUPLICATE",
      "REC-CANCEL-LATE",
      "REC-TERMINATION",
    ]) {
      expect(readiness).toContain(evidenceId);
    }

    for (const recoveryState of [
      "permission_denied",
      "queued_offline",
      "retryable_failure",
      "terminal_failure",
      "cancelled",
      "unrecoverable",
    ]) {
      expect(stateMachine).toContain(`\`${recoveryState}\``);
    }

    expect(recorder).toContain("never auto-resume");
    expect(recorder).toContain("Recovery never triggers upload");
  });

  test("defines privacy lifecycle and downstream ownership", () => {
    const readiness = readFeatureFile("checklists/implementation-readiness.md");
    const specification = readFeatureFile("spec.md");
    const transcription = readFeatureFile("contracts/transcription-boundary.md");

    for (const evidenceId of [
      "PRIV-LOCAL",
      "PRIV-BACKEND",
      "PRIV-PROVIDER",
      "PRIV-CREDENTIALS",
      "PRIV-LOGGING",
      "SCOPE-DEFERRED",
      "SUCCESS-OWNERSHIP",
    ]) {
      expect(readiness).toContain(evidenceId);
    }

    expect(specification).toContain("delete local source audio after successful");
    expect(transcription).toContain("never communicates with OpenAI directly");
    expect(transcription).toContain("within 24 hours");
    expect(transcription).toContain("Raw audio, transcript text, credentials");
  });
});
