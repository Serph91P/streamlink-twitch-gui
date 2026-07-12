import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";

import { BrowserBackend } from "../api/backend";
import { createTwitchQueries, twitchQueryKeys } from "./twitch";

describe("Twitch queries", () => {
  it("passes cursors and AbortSignals through infinite queries", async () => {
    const topGames = vi.fn(async (cursor?: string, signal?: AbortSignal) => ({
      items: [],
      nextCursor: cursor ? undefined : "next",
      signal,
    }));
    const backend = new BrowserBackend({ topGames });
    const queries = createTwitchQueries(backend, new QueryClient());
    const options = queries.topGames();
    const controller = new AbortController();

    const first = await options.queryFn!({
      pageParam: undefined,
      signal: controller.signal,
      queryKey: options.queryKey,
      direction: "forward",
      meta: undefined,
      client: new QueryClient(),
    });
    expect(options.getNextPageParam(first, [], undefined, [])).toBe("next");
    expect(topGames).toHaveBeenCalledWith(undefined, controller.signal);
  });

  it("marks remote data stale and removes it after sign-out", async () => {
    const signOut = vi.fn(async () => undefined);
    const backend = new BrowserBackend({ signOut });
    const queryClient = new QueryClient();
    const queries = createTwitchQueries(backend, queryClient);
    queryClient.setQueryData(twitchQueryKeys.session, { status: "anonymous" });
    queryClient.setQueryData([...twitchQueryKeys.all, "private"], ["cached"]);

    expect(queries.session.staleTime).toBe(0);
    await queries.signOut();

    expect(signOut).toHaveBeenCalledOnce();
    expect(
      queryClient.getQueriesData({ queryKey: twitchQueryKeys.all }),
    ).toEqual([]);
  });
});
