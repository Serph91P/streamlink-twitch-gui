export interface TwitchUser {
  id: string;
  login: string;
  displayName: string;
  profileImageUrl: string;
}

export type TwitchSession =
  | { status: "anonymous" }
  | { status: "authenticated"; user: TwitchUser; expiresAt: string };

export interface TwitchLoginChallenge {
  verificationUri: string;
  userCode: string;
  expiresInSeconds: number;
  pollingIntervalSeconds: number;
}

export interface TwitchPage<T> {
  items: T[];
  nextCursor?: string;
  rateLimit?: {
    limit?: number;
    remaining?: number;
    resetAtEpochSeconds?: number;
  };
}

export interface TwitchStream {
  id: string;
  userId: string;
  userLogin: string;
  userName: string;
  gameId: string;
  gameName: string;
  title: string;
  viewerCount: number;
  startedAt: string;
  thumbnailUrl: string;
  isMature: boolean;
}

export interface FollowedChannel {
  broadcasterId: string;
  broadcasterLogin: string;
  broadcasterName: string;
  followedAt: string;
}

export interface TwitchGame {
  id: string;
  name: string;
  boxArtUrl: string;
  igdbId?: string;
}

export interface TwitchSearchChannel {
  broadcasterLanguage: string;
  broadcasterLogin: string;
  displayName: string;
  gameId: string;
  gameName: string;
  id: string;
  isLive: boolean;
  tags: string[];
  thumbnailUrl: string;
  title: string;
  startedAt: string;
}

function record(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }

  return value as Record<string, unknown>;
}

function text(value: unknown, name: string): string {
  if (typeof value !== "string") {
    throw new TypeError(`${name} must be a string`);
  }

  return value;
}

export function parseTwitchSession(value: unknown): TwitchSession {
  const session = record(value, "Twitch session");
  if (session.status === "anonymous") {
    return { status: "anonymous" };
  }
  if (session.status !== "authenticated") {
    throw new TypeError("Twitch session status is invalid");
  }

  const user = record(session.user, "Twitch user");
  return {
    status: "authenticated",
    user: {
      id: text(user.id, "Twitch user id"),
      login: text(user.login, "Twitch user login"),
      displayName: text(user.displayName, "Twitch user displayName"),
      profileImageUrl: text(
        user.profileImageUrl,
        "Twitch user profileImageUrl",
      ),
    },
    expiresAt: text(session.expiresAt, "Twitch session expiresAt"),
  };
}

export function parseTwitchLoginChallenge(
  value: unknown,
): TwitchLoginChallenge {
  const challenge = record(value, "Twitch login challenge");
  const expiresInSeconds = challenge.expiresInSeconds;
  const pollingIntervalSeconds = challenge.pollingIntervalSeconds;
  if (
    !Number.isSafeInteger(expiresInSeconds) ||
    (expiresInSeconds as number) <= 0 ||
    !Number.isSafeInteger(pollingIntervalSeconds) ||
    (pollingIntervalSeconds as number) <= 0
  ) {
    throw new TypeError("Twitch login challenge timing is invalid");
  }
  return {
    verificationUri: text(challenge.verificationUri, "Twitch verification URI"),
    userCode: text(challenge.userCode, "Twitch user code"),
    expiresInSeconds: expiresInSeconds as number,
    pollingIntervalSeconds: pollingIntervalSeconds as number,
  };
}
