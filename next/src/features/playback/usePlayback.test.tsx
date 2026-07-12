import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlaybackBackend } from "../../api/backend";
import { usePlayback } from "./usePlayback";
import { variants } from "./QualityPicker.test";

describe("usePlayback", () => {
  it("stops and relaunches when selection changes while running", async () => {
    const calls: string[] = [];
    const backend: PlaybackBackend = {
      inspectStreams: vi.fn(async () => ({
        variants,
        supportsCodecSelection: true,
      })),
      launchStream: vi.fn(async (request) => {
        calls.push(`launch:${request.variantName}`);
        return { status: "running" as const, diagnostics: [] };
      }),
      stopStream: vi.fn(async () => {
        calls.push("stop");
        return { status: "stopped" as const, diagnostics: [] };
      }),
    };
    const { result } = renderHook(() =>
      usePlayback(backend, "https://twitch.tv/signalnoise", variants),
    );

    await act(() => result.current.play());
    await act(() => result.current.select("1440p60-av1"));

    expect(calls).toEqual([
      "launch:1440p60-hevc",
      "stop",
      "launch:1440p60-av1",
    ]);
    await waitFor(() => expect(result.current.status).toBe("running"));
  });

  it("turns backend failures into actionable diagnostics", async () => {
    const backend: PlaybackBackend = {
      inspectStreams: vi.fn(),
      launchStream: vi.fn(async () => {
        throw new Error("Player executable was not found");
      }),
      stopStream: vi.fn(),
    };
    const { result } = renderHook(() =>
      usePlayback(backend, "https://twitch.tv/signalnoise", variants),
    );
    await act(() => result.current.play());
    expect(result.current.status).toBe("error");
    expect(result.current.diagnostics).toContain(
      "Player executable was not found",
    );
  });

  it("launches with only the selected variant codec", async () => {
    const backend: PlaybackBackend = {
      inspectStreams: vi.fn(),
      launchStream: vi.fn(async () => ({
        status: "running" as const,
        diagnostics: [],
      })),
      stopStream: vi.fn(),
    };
    const { result } = renderHook(() =>
      usePlayback(backend, "https://twitch.tv/signalnoise", variants),
    );

    await act(() => result.current.select("1440p60-av1"));
    await act(() => result.current.play());

    expect(backend.launchStream).toHaveBeenCalledWith(
      expect.objectContaining({ codecs: ["av1"] }),
    );
  });
});
