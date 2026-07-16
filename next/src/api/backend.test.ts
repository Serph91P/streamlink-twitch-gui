import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { BrowserBackend, TauriBackend } from "./backend";

beforeEach(() => invoke.mockReset());

describe("browser backend", () => {
  it("provides typed cursor pages and honors cancellation", async () => {
    const backend = new BrowserBackend({
      topGames: async (cursor) => ({
        items: [{ id: "1", name: cursor ?? "first", boxArtUrl: "image" }],
        nextCursor: cursor ? undefined : "page-2",
      }),
    });

    const first = await backend.topGames();
    const second = await backend.topGames(first.nextCursor);
    expect(second.items[0]?.name).toBe("page-2");

    const controller = new AbortController();
    controller.abort();
    await expect(
      backend.topGames(undefined, controller.signal),
    ).rejects.toThrow(/aborted/i);
  });

  it("never exposes credential fields in its public values", async () => {
    const backend = new BrowserBackend();
    const values = [
      await backend.getSession(),
      await backend.beginTwitchLogin("public-test-attempt"),
    ];

    expect(JSON.stringify(values)).not.toMatch(
      /accessToken|refreshToken|deviceCode|clientSecret/i,
    );
  });
});

describe("tauri legacy migration backend", () => {
  it("uses only an explicit legacy export and never the new webview localStorage", async () => {
    const snapshot = {
      settings: '{"settings":{"records":{}}}',
    };
    const storageRead = vi
      .spyOn(Storage.prototype, "getItem")
      .mockImplementation(() => {
        throw new Error("new webview localStorage must not be read");
      });
    invoke.mockResolvedValue({ status: "ready" });

    await new TauriBackend().previewLegacyMigration(snapshot);

    expect(storageRead).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith("preview_legacy_migration", {
      snapshot,
    });
    storageRead.mockRestore();
  });
});

describe("tauri prerequisite status backend", () => {
  it("probes player availability without sending or receiving a path", async () => {
    invoke.mockResolvedValue({ state: "configuredUnavailable" });

    await expect(new TauriBackend().getPlayerStatus()).resolves.toEqual({
      state: "configuredUnavailable",
    });
    expect(invoke).toHaveBeenCalledWith("get_player_status", undefined);
  });
});

describe("tauri Twitch login cancellation", () => {
  it("uses one attempt ID across begin, poll, and idempotent cancellation", async () => {
    invoke
      .mockResolvedValueOnce({
        verificationUri: "https://www.twitch.tv/activate",
        userCode: "ABCD-EFGH",
        expiresInSeconds: 600,
        pollingIntervalSeconds: 5,
      })
      .mockResolvedValueOnce({ status: "anonymous" })
      .mockResolvedValue(undefined);
    const backend = new TauriBackend();

    await backend.beginTwitchLogin("attempt-1");
    await backend.pollTwitchLogin("attempt-1");
    await backend.cancelTwitchLogin("attempt-1");
    await backend.cancelTwitchLogin("attempt-1");

    expect(invoke.mock.calls).toEqual([
      ["begin_twitch_login", { attemptId: "attempt-1" }],
      ["poll_twitch_login", { attemptId: "attempt-1" }],
      ["cancel_twitch_login", { attemptId: "attempt-1" }],
      ["cancel_twitch_login", { attemptId: "attempt-1" }],
    ]);
  });
});

describe("tauri error boundary", () => {
  it.each([
    ["getSession", "Helix authentication failed"],
    ["inspectStreams", "Streamlink was not found"],
  ] as const)(
    "normalizes string rejections from %s",
    async (operation, message) => {
      invoke.mockRejectedValueOnce(message);
      const backend = new TauriBackend();

      const result =
        operation === "getSession"
          ? backend.getSession()
          : backend.inspectStreams("https://twitch.tv/example");

      await expect(result).rejects.toEqual(expect.any(Error));
      await expect(result).rejects.toThrow(message);
    },
  );

  it("does not expose fields from unknown rejection objects", async () => {
    invoke.mockRejectedValueOnce({
      message: "request failed",
      accessToken: "component-secret-token",
    });

    const error = await new TauriBackend()
      .loadSettings()
      .catch((reason) => reason);

    expect(error).toEqual(expect.any(Error));
    expect(error).toHaveProperty("message", "Desktop command failed");
    expect(String(error)).not.toContain("component-secret-token");
  });
});
