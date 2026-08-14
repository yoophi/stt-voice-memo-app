import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

import { App } from "@/app/App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("fetch", vi.fn());
});

test("renders the ready mobile foundation without pretending voice features exist", () => {
  render(<App />);

  expect(screen.getByRole("heading", { level: 1, name: "STT Voice Memo" })).toBeVisible();
  expect(screen.getByText("모바일 음성 메모를 위한 앱 기반이 준비되었습니다.")).toBeVisible();
  expect(screen.getByText("녹음과 음성 변환 기능은 다음 단계에서 제공됩니다.")).toBeVisible();
  expect(screen.queryByRole("button")).not.toBeInTheDocument();
});

test("starts without network or native side effects", () => {
  render(<App />);

  expect(fetch).not.toHaveBeenCalled();
  expect(invoke).not.toHaveBeenCalled();
});
