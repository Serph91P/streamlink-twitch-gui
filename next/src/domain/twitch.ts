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
