import {
  infiniteQueryOptions,
  queryOptions,
  type QueryClient,
} from "@tanstack/react-query";

import type { TwitchBackend } from "../api/backend";

export const twitchQueryKeys = {
  all: ["twitch"] as const,
  session: ["twitch", "session"] as const,
};

export function createTwitchQueries(
  backend: TwitchBackend,
  queryClient: QueryClient,
) {
  return {
    session: queryOptions({
      queryKey: twitchQueryKeys.session,
      queryFn: ({ signal }) => backend.getSession(signal),
      staleTime: 0,
    }),
    topGames: () =>
      infiniteQueryOptions({
        queryKey: [...twitchQueryKeys.all, "top-games"] as const,
        queryFn: ({ pageParam, signal }) => backend.topGames(pageParam, signal),
        initialPageParam: undefined as string | undefined,
        getNextPageParam: (page) => page.nextCursor,
        staleTime: 30_000,
      }),
    followedStreams: (userId: string) =>
      infiniteQueryOptions({
        queryKey: [...twitchQueryKeys.all, "followed-streams", userId] as const,
        queryFn: ({ pageParam, signal }) =>
          backend.followedStreams(userId, pageParam, signal),
        initialPageParam: undefined as string | undefined,
        getNextPageParam: (page) => page.nextCursor,
        staleTime: 30_000,
      }),
    searchChannels: (search: string) =>
      infiniteQueryOptions({
        queryKey: [...twitchQueryKeys.all, "search-channels", search] as const,
        queryFn: ({ pageParam, signal }) =>
          backend.searchChannels(search, pageParam, signal),
        initialPageParam: undefined as string | undefined,
        getNextPageParam: (page) => page.nextCursor,
        staleTime: 30_000,
      }),
    async signOut(): Promise<void> {
      await backend.signOut();
      queryClient.removeQueries({ queryKey: twitchQueryKeys.all });
    },
  };
}
