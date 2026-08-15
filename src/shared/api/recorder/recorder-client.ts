import { addPluginListener, invoke } from "@tauri-apps/api/core";

export type PermissionState = "undetermined" | "granted" | "denied" | "restricted";
export type RecordingState =
  "idle" | "recording" | "paused" | "finalizing" | "finalized" | "cancelled" | "failed";
export type FinalizationReason =
  "userStop" | "interruption" | "routeChange" | "foregroundExit" | "mediaServicesReset";
export type CleanupOutcome = "removed" | "notFound" | "pending" | "failed";

export interface PermissionOutcome {
  state: PermissionState;
  canRequest: boolean;
  canOpenSettings: boolean;
}

export interface RecordingSession {
  sessionId?: string;
  state: RecordingState;
  startedAtMs?: number;
  durationMs: number;
  terminalReason?: FinalizationReason;
}

export interface FinalizedRecording {
  artifactId: string;
  sessionId: string;
  mimeType: "audio/mp4";
  fileExtension: "m4a";
  durationMs: number;
  byteLength: number;
  sampleRateHz: number;
  channelCount: number;
  sha256: string;
  finalizationReason: FinalizationReason;
}

export type RecorderEventRecording = FinalizedRecording;

export interface RecorderEvent {
  eventId: string;
  sessionId: string;
  sequence: number;
  state: RecordingState;
  reason?: FinalizationReason;
  recording?: RecorderEventRecording;
  cleanup?: CleanupOutcome;
}

const command = (name: string) => `plugin:recorder|${name}`;

export async function permissionStatus(): Promise<PermissionOutcome> {
  return await invoke<PermissionOutcome>(command("permission_status"));
}

export async function requestPermission(): Promise<PermissionOutcome> {
  return await invoke<PermissionOutcome>(command("request_permission"));
}

export async function recorderStatus(sessionId?: string): Promise<RecordingSession> {
  return await invoke<RecordingSession>(command("recorder_status"), {
    payload: { sessionId },
  });
}

export async function startRecording(sessionId: string): Promise<RecordingSession> {
  return await sessionCommand("start", sessionId);
}

export async function pauseRecording(sessionId: string): Promise<RecordingSession> {
  return await sessionCommand("pause", sessionId);
}

export async function resumeRecording(sessionId: string): Promise<RecordingSession> {
  return await sessionCommand("resume", sessionId);
}

export async function stopRecording(sessionId: string): Promise<FinalizedRecording> {
  const value = await invoke<unknown>(command("stop"), {
    payload: { sessionId, reason: "userStop" },
  });
  return sanitizeFinalizedRecording(value);
}

export async function cancelRecording(sessionId: string): Promise<CleanupOutcome> {
  return await invoke<CleanupOutcome>(command("cancel"), {
    payload: { sessionId },
  });
}

export async function onRecorderEvent(
  handler: (event: RecorderEvent) => void,
): Promise<() => Promise<void>> {
  const listener = await addPluginListener<unknown>("recorder", "recorderEvent", (payload) => {
    handler(sanitizeRecorderEvent(payload));
  });
  return async () => await listener.unregister();
}

async function sessionCommand(name: "start" | "pause" | "resume", sessionId: string) {
  return await invoke<RecordingSession>(command(name), {
    payload: { sessionId },
  });
}

function sanitizeFinalizedRecording(value: unknown): FinalizedRecording {
  if (!isRecord(value)) {
    throw new TypeError("invalid finalized recording response");
  }
  const recording: FinalizedRecording = {
    artifactId: stringField(value, "artifactId"),
    sessionId: stringField(value, "sessionId"),
    mimeType: literalField(value, "mimeType", "audio/mp4"),
    fileExtension: literalField(value, "fileExtension", "m4a"),
    durationMs: positiveNumberField(value, "durationMs"),
    byteLength: positiveNumberField(value, "byteLength"),
    sampleRateHz: positiveNumberField(value, "sampleRateHz"),
    channelCount: positiveNumberField(value, "channelCount"),
    sha256: stringField(value, "sha256"),
    finalizationReason: finalizationReasonField(value, "finalizationReason"),
  };
  if (!/^[0-9a-f]{64}$/.test(recording.sha256)) {
    throw new TypeError("invalid finalized recording checksum");
  }
  return recording;
}

function sanitizeRecorderEvent(value: unknown): RecorderEvent {
  if (!isRecord(value)) {
    throw new TypeError("invalid recorder event");
  }
  const event: RecorderEvent = {
    eventId: stringField(value, "eventId"),
    sessionId: stringField(value, "sessionId"),
    sequence: positiveNumberField(value, "sequence"),
    state: recordingStateField(value, "state"),
  };
  if (value.reason !== undefined && value.reason !== null) {
    event.reason = finalizationReasonField(value, "reason");
  }
  if (isRecord(value.recording)) {
    event.recording = sanitizeEventRecording(value.recording);
    if (event.recording.sessionId !== event.sessionId) {
      throw new TypeError("recorder event recording belongs to another session");
    }
  }
  if (value.cleanup !== undefined && value.cleanup !== null) {
    event.cleanup = cleanupField(value, "cleanup");
  }
  return event;
}

function sanitizeEventRecording(value: Record<string, unknown>): RecorderEventRecording {
  const recording: RecorderEventRecording = {
    artifactId: stringField(value, "artifactId"),
    sessionId: stringField(value, "sessionId"),
    mimeType: literalField(value, "mimeType", "audio/mp4"),
    fileExtension: literalField(value, "fileExtension", "m4a"),
    durationMs: positiveNumberField(value, "durationMs"),
    byteLength: positiveNumberField(value, "byteLength"),
    sampleRateHz: positiveNumberField(value, "sampleRateHz"),
    channelCount: positiveNumberField(value, "channelCount"),
    sha256: stringField(value, "sha256"),
    finalizationReason: finalizationReasonField(value, "finalizationReason"),
  };
  if (!/^[0-9a-f]{64}$/.test(recording.sha256)) {
    throw new TypeError("invalid recorder event checksum");
  }
  return recording;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function stringField(value: Record<string, unknown>, key: string): string {
  const field = value[key];
  if (typeof field !== "string" || field.length === 0) {
    throw new TypeError(`invalid ${key}`);
  }
  return field;
}

function positiveNumberField(value: Record<string, unknown>, key: string): number {
  const field = value[key];
  if (typeof field !== "number" || !Number.isFinite(field) || field <= 0) {
    throw new TypeError(`invalid ${key}`);
  }
  return field;
}

function literalField<const T extends string>(
  value: Record<string, unknown>,
  key: string,
  expected: T,
): T {
  if (value[key] !== expected) {
    throw new TypeError(`invalid ${key}`);
  }
  return expected;
}

function finalizationReasonField(value: Record<string, unknown>, key: string): FinalizationReason {
  const reason = value[key];
  if (
    reason !== "userStop" &&
    reason !== "interruption" &&
    reason !== "routeChange" &&
    reason !== "foregroundExit" &&
    reason !== "mediaServicesReset"
  ) {
    throw new TypeError(`invalid ${key}`);
  }
  return reason;
}

function recordingStateField(value: Record<string, unknown>, key: string): RecordingState {
  const state = value[key];
  if (
    state !== "idle" &&
    state !== "recording" &&
    state !== "paused" &&
    state !== "finalizing" &&
    state !== "finalized" &&
    state !== "cancelled" &&
    state !== "failed"
  ) {
    throw new TypeError(`invalid ${key}`);
  }
  return state;
}

function cleanupField(value: Record<string, unknown>, key: string): CleanupOutcome {
  const cleanup = value[key];
  if (
    cleanup !== "removed" &&
    cleanup !== "notFound" &&
    cleanup !== "pending" &&
    cleanup !== "failed"
  ) {
    throw new TypeError(`invalid ${key}`);
  }
  return cleanup;
}
