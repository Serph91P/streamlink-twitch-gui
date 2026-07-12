import { describe, expect, it } from "vitest";

import { BrowserBackend } from "./backend";

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
