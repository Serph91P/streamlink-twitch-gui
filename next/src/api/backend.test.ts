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
      await backend.beginTwitchLogin(),
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
