import { addPluginListener, invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  cancelRecording,
  pauseRecording,
  onRecorderEvent,
  permissionStatus,
  resumeRecording,
  startRecording,
  stopRecording,
} from "./recorder-client";

vi.mock("@tauri-apps/api/core", () => ({
  addPluginListener: vi.fn(),
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedAddPluginListener = vi.mocked(addPluginListener);
const sessionId = "550e8400-e29b-41d4-a716-446655440000";

describe("recorder client", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedAddPluginListener.mockReset();
  });

  it("uses only the recorder plugin command namespace", async () => {
    mockedInvoke.mockResolvedValue({
      sessionId,
      state: "recording",
      durationMs: 0,
    });

    await startRecording(sessionId);
    await pauseRecording(sessionId);
    await resumeRecording(sessionId);

    expect(mockedInvoke.mock.calls).toEqual([
      ["plugin:recorder|start", { payload: { sessionId } }],
      ["plugin:recorder|pause", { payload: { sessionId } }],
      ["plugin:recorder|resume", { payload: { sessionId } }],
    ]);
  });

  it("uses the permission command without a payload", async () => {
    mockedInvoke.mockResolvedValue({
      state: "granted",
      canRequest: false,
      canOpenSettings: false,
    });

    await permissionStatus();

    expect(mockedInvoke).toHaveBeenCalledWith("plugin:recorder|permission_status");
  });

  it("maps cancel cleanup outcomes without exposing a locator", async () => {
    mockedInvoke.mockResolvedValue("pending");

    const cleanup = await cancelRecording(sessionId);

    expect(cleanup).toBe("pending");
    expect(mockedInvoke).toHaveBeenCalledWith("plugin:recorder|cancel", {
      payload: { sessionId },
    });
  });

  it("redacts unexpected native file locator fields from stop results", async () => {
    mockedInvoke.mockResolvedValue({
      artifactId: "c56a4180-65aa-42ec-a945-5fd21dec0538",
      sessionId,
      mimeType: "audio/mp4",
      fileExtension: "m4a",
      durationMs: 750,
      byteLength: 128,
      sampleRateHz: 44_100,
      channelCount: 1,
      sha256: "a".repeat(64),
      finalizationReason: "userStop",
      fileUri: "file:///private/secret.m4a",
    });

    const recording = await stopRecording(sessionId);

    expect(recording).not.toHaveProperty("fileUri");
    expect(recording.mimeType).toBe("audio/mp4");
    expect(mockedInvoke).toHaveBeenCalledWith("plugin:recorder|stop", {
      payload: { sessionId, reason: "userStop" },
    });
  });

  it("subscribes to plugin-scoped sanitized events", async () => {
    const unregister = vi.fn();
    mockedAddPluginListener.mockImplementation(async (_plugin, _event, callback) => {
      callback({
        eventId: "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        sessionId,
        sequence: 2,
        state: "finalized",
        reason: "interruption",
        recording: {
          artifactId: "c56a4180-65aa-42ec-a945-5fd21dec0538",
          mimeType: "audio/mp4",
          fileExtension: "m4a",
          durationMs: 750,
          sampleRateHz: 44_100,
          channelCount: 1,
          sha256: "a".repeat(64),
          finalizationReason: "interruption",
          fileUri: "file:///private/secret.m4a",
        },
      });
      return { unregister } as never;
    });
    const handler = vi.fn();

    const remove = await onRecorderEvent(handler);

    expect(mockedAddPluginListener).toHaveBeenCalledWith(
      "recorder",
      "recorderEvent",
      expect.any(Function),
    );
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({
        eventId: "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        sequence: 2,
        reason: "interruption",
      }),
    );
    expect(handler.mock.calls[0]?.[0].recording).not.toHaveProperty("fileUri");
    await remove();
    expect(unregister).toHaveBeenCalledOnce();
  });
});
