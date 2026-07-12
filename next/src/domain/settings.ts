import type {
  CodecPreference,
  QualityConstraints,
  StreamCodec,
} from "./stream";

export interface PlayerSettings {
  path?: string;
  arguments: string[];
}

export interface Settings {
  schemaVersion: 1;
  streamlinkPath?: string;
  quality: QualityConstraints;
  codecPreference: CodecPreference;
  player: PlayerSettings;
  theme: "system" | "dark" | "light";
  language: string;
  notifications: { liveChannels: boolean; playbackErrors: boolean };
  hotkey: { enabled: boolean; accelerator: string };
}

function record(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }

  return value as Record<string, unknown>;
}

function exactKeys(
  input: Record<string, unknown>,
  allowed: readonly string[],
  name: string,
): void {
  const unknown = Object.keys(input).find((key) => !allowed.includes(key));
  if (unknown) {
    throw new TypeError(`unknown ${name} field: ${unknown}`);
  }
}

function optionalPositiveInteger(
  value: unknown,
  name: string,
): number | undefined {
  if (value === undefined) {
    return undefined;
  }
  if (!Number.isSafeInteger(value) || (value as number) <= 0) {
    throw new TypeError(`${name} must be a positive integer`);
  }

  return value as number;
}

function codec(value: unknown, name: string): StreamCodec {
  if (
    value !== "h264" &&
    value !== "h265" &&
    value !== "av1" &&
    value !== "unknown"
  ) {
    throw new TypeError(`${name} is not a supported codec preference`);
  }

  return value;
}

export function parseSettings(value: unknown): Settings {
  const input = record(value, "settings");
  exactKeys(
    input,
    [
      "schemaVersion",
      "streamlinkPath",
      "quality",
      "codecPreference",
      "player",
      "theme",
      "language",
      "notifications",
      "hotkey",
    ],
    "settings",
  );
  if (input.schemaVersion !== 1)
    throw new TypeError("unsupported settings schema");

  const quality = record(input.quality, "quality");
  exactKeys(quality, ["preference", "maximumHeight", "maximumFps"], "quality");
  if (
    quality.preference !== "best" &&
    quality.preference !== "worst" &&
    quality.preference !== "audioOnly"
  ) {
    throw new TypeError("quality preference is invalid");
  }

  const codecPreference = record(input.codecPreference, "codec preference");
  exactKeys(codecPreference, ["allowed", "preferred"], "codec preference");
  if (!Array.isArray(codecPreference.allowed)) {
    throw new TypeError("allowed codecs must be an array");
  }

  const player = record(input.player, "player");
  exactKeys(player, ["path", "arguments"], "player");
  if (!Array.isArray(player.arguments)) {
    throw new TypeError("player arguments must be an array");
  }
  if (player.path !== undefined && typeof player.path !== "string") {
    throw new TypeError("player path must be a string");
  }
  const notifications = record(input.notifications, "notifications");
  exactKeys(notifications, ["liveChannels", "playbackErrors"], "notifications");
  const hotkey = record(input.hotkey, "hotkey");
  exactKeys(hotkey, ["enabled", "accelerator"], "hotkey");
  if (
    typeof notifications.liveChannels !== "boolean" ||
    typeof notifications.playbackErrors !== "boolean" ||
    typeof hotkey.enabled !== "boolean" ||
    typeof hotkey.accelerator !== "string" ||
    typeof input.language !== "string" ||
    !["system", "dark", "light"].includes(input.theme as string)
  ) {
    throw new TypeError("desktop settings are invalid");
  }

  const parsed: Settings = {
    schemaVersion: 1,
    quality: {
      preference: quality.preference,
    },
    codecPreference: {
      allowed: codecPreference.allowed.map((item) =>
        codec(item, "allowed codec"),
      ),
    },
    player: {
      arguments: player.arguments.map((argument) => {
        if (typeof argument !== "string") {
          throw new TypeError("player argument must be a string");
        }
        return argument;
      }),
    },
    theme: input.theme as Settings["theme"],
    language: input.language,
    notifications: {
      liveChannels: notifications.liveChannels,
      playbackErrors: notifications.playbackErrors,
    },
    hotkey: {
      enabled: hotkey.enabled,
      accelerator: hotkey.accelerator,
    },
  };

  const maximumHeight = optionalPositiveInteger(
    quality.maximumHeight,
    "maximumHeight",
  );
  const maximumFps = optionalPositiveInteger(quality.maximumFps, "maximumFps");
  if (maximumHeight !== undefined) parsed.quality.maximumHeight = maximumHeight;
  if (maximumFps !== undefined) parsed.quality.maximumFps = maximumFps;
  if (codecPreference.preferred !== undefined) {
    parsed.codecPreference.preferred = codec(
      codecPreference.preferred,
      "preferred codec",
    );
  }
  if (player.path !== undefined) parsed.player.path = player.path;
  if (input.streamlinkPath !== undefined) {
    if (typeof input.streamlinkPath !== "string") {
      throw new TypeError("streamlinkPath must be a string");
    }
    parsed.streamlinkPath = input.streamlinkPath;
  }

  return parsed;
}
