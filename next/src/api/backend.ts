import { invoke } from "@tauri-apps/api/core";

import {
  parseTwitchLoginChallenge,
  parseTwitchSession,
  type FollowedChannel,
  type TwitchGame,
  type TwitchLoginChallenge,
  type TwitchPage,
  type TwitchSearchChannel,
  type TwitchSession,
  type TwitchStream,
  type TwitchUser,
} from "../domain/twitch";

export interface TwitchBackend {
  getSession(signal?: AbortSignal): Promise<TwitchSession>;
  beginTwitchLogin(signal?: AbortSignal): Promise<TwitchLoginChallenge>;
  pollTwitchLogin(signal?: AbortSignal): Promise<TwitchSession>;
  signOut(signal?: AbortSignal): Promise<void>;
  users(
    logins: string[],
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchUser>>;
  streams(
    userId?: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>>;
  followedStreams(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>>;
  followedChannels(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<FollowedChannel>>;
  topGames(
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>>;
  searchChannels(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchSearchChannel>>;
  searchCategories(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>>;
}

type BackendOverrides = Partial<{
  [Key in keyof TwitchBackend]: TwitchBackend[Key];
}>;

const emptyPage = <T>(): TwitchPage<T> => ({ items: [] });

function throwIfAborted(signal?: AbortSignal): void {
  signal?.throwIfAborted();
}

// Tauri invoke has no cancellation handle. Aborting only detaches this caller;
// backend work continues until the command completes.
async function detachOnAbort<T>(
  operation: Promise<T>,
  signal?: AbortSignal,
): Promise<T> {
  throwIfAborted(signal);
  if (!signal) return operation;

  let rejectAbort: ((reason: unknown) => void) | undefined;
  const aborted = new Promise<never>((_, reject) => {
    rejectAbort = reject;
  });
  const onAbort = () => rejectAbort?.(signal.reason);
  signal.addEventListener("abort", onAbort, { once: true });
  try {
    return await Promise.race([operation, aborted]);
  } finally {
    signal.removeEventListener("abort", onAbort);
  }
}

export class TauriBackend implements TwitchBackend {
  async getSession(signal?: AbortSignal): Promise<TwitchSession> {
    return parseTwitchSession(
      await detachOnAbort(invoke("get_twitch_session"), signal),
    );
  }

  async beginTwitchLogin(signal?: AbortSignal): Promise<TwitchLoginChallenge> {
    return parseTwitchLoginChallenge(
      await detachOnAbort(invoke("begin_twitch_login"), signal),
    );
  }

  async pollTwitchLogin(signal?: AbortSignal): Promise<TwitchSession> {
    return parseTwitchSession(
      await detachOnAbort(invoke("poll_twitch_login"), signal),
    );
  }

  async signOut(signal?: AbortSignal): Promise<void> {
    await detachOnAbort(invoke("sign_out_twitch"), signal);
  }

  users(
    logins: string[],
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchUser>> {
    return detachOnAbort(invoke("twitch_users", { logins }), signal);
  }

  streams(
    userId?: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>> {
    return detachOnAbort(invoke("twitch_streams", { userId, cursor }), signal);
  }

  followedStreams(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>> {
    return detachOnAbort(
      invoke("twitch_followed_streams", { userId, cursor }),
      signal,
    );
  }

  followedChannels(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<FollowedChannel>> {
    return detachOnAbort(
      invoke("twitch_followed_channels", { userId, cursor }),
      signal,
    );
  }

  topGames(
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>> {
    return detachOnAbort(invoke("twitch_top_games", { cursor }), signal);
  }

  searchChannels(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchSearchChannel>> {
    return detachOnAbort(
      invoke("twitch_search_channels", { query, cursor }),
      signal,
    );
  }

  searchCategories(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>> {
    return detachOnAbort(
      invoke("twitch_search_categories", { query, cursor }),
      signal,
    );
  }
}

export class BrowserBackend implements TwitchBackend {
  constructor(private readonly overrides: BackendOverrides = {}) {}

  async getSession(signal?: AbortSignal): Promise<TwitchSession> {
    throwIfAborted(signal);
    return this.overrides.getSession?.(signal) ?? { status: "anonymous" };
  }

  async beginTwitchLogin(signal?: AbortSignal): Promise<TwitchLoginChallenge> {
    throwIfAborted(signal);
    return (
      this.overrides.beginTwitchLogin?.(signal) ?? {
        verificationUri: "https://www.twitch.tv/activate",
        userCode: "TEST-CODE",
        expiresInSeconds: 600,
        pollingIntervalSeconds: 5,
      }
    );
  }

  async pollTwitchLogin(signal?: AbortSignal): Promise<TwitchSession> {
    throwIfAborted(signal);
    return this.overrides.pollTwitchLogin?.(signal) ?? { status: "anonymous" };
  }

  async signOut(signal?: AbortSignal): Promise<void> {
    throwIfAborted(signal);
    await this.overrides.signOut?.(signal);
  }

  async users(
    logins: string[],
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchUser>> {
    throwIfAborted(signal);
    return this.overrides.users?.(logins, signal) ?? emptyPage();
  }

  async streams(
    userId?: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>> {
    throwIfAborted(signal);
    return this.overrides.streams?.(userId, cursor, signal) ?? emptyPage();
  }

  async followedStreams(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchStream>> {
    throwIfAborted(signal);
    return (
      this.overrides.followedStreams?.(userId, cursor, signal) ?? emptyPage()
    );
  }

  async followedChannels(
    userId: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<FollowedChannel>> {
    throwIfAborted(signal);
    return (
      this.overrides.followedChannels?.(userId, cursor, signal) ?? emptyPage()
    );
  }

  async topGames(
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>> {
    throwIfAborted(signal);
    return this.overrides.topGames?.(cursor, signal) ?? emptyPage();
  }

  async searchChannels(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchSearchChannel>> {
    throwIfAborted(signal);
    return (
      this.overrides.searchChannels?.(query, cursor, signal) ?? emptyPage()
    );
  }

  async searchCategories(
    query: string,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<TwitchPage<TwitchGame>> {
    throwIfAborted(signal);
    return (
      this.overrides.searchCategories?.(query, cursor, signal) ?? emptyPage()
    );
  }
}
