export type StreamCodec = "h264" | "h265" | "av1" | "unknown";

export interface StreamResolution {
  width: number;
  height: number;
}

export interface StreamVariant {
  name: string;
  resolution?: StreamResolution;
  fps?: number;
  codec?: StreamCodec;
  bitrateKbps?: number;
  aliases: string[];
}

export interface CodecPreference {
  allowed: StreamCodec[];
  preferred?: StreamCodec;
}

export interface QualityConstraints {
  preference: "best" | "worst" | "audioOnly";
  maximumHeight?: number;
  maximumFps?: number;
}

export interface StreamCapabilities {
  variants: StreamVariant[];
  supportsCodecSelection: boolean;
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

function positiveNumber(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    throw new TypeError(`${name} must be a positive number`);
  }

  return value;
}

export function parseCodec(value: unknown): StreamCodec {
  if (value === "h264" || value === "h265" || value === "av1") {
    return value;
  }

  return "unknown";
}

export function parseStreamCapabilities(value: unknown): StreamCapabilities {
  const input = record(value, "stream capabilities");

  if (!Array.isArray(input.variants)) {
    throw new TypeError("stream capabilities variants must be an array");
  }
  if (typeof input.supportsCodecSelection !== "boolean") {
    throw new TypeError("supportsCodecSelection must be a boolean");
  }

  return {
    variants: input.variants.map((item, index) => {
      const variant = record(item, `variant ${index}`);
      const aliases = variant.aliases;
      if (!Array.isArray(aliases)) {
        throw new TypeError(`variant ${index} aliases must be an array`);
      }

      const parsed: StreamVariant = {
        name: text(variant.name, `variant ${index} name`),
        aliases: aliases.map((alias) => text(alias, `variant ${index} alias`)),
      };

      if (variant.resolution !== undefined) {
        const resolution = record(
          variant.resolution,
          `variant ${index} resolution`,
        );
        parsed.resolution = {
          width: positiveNumber(resolution.width, "resolution width"),
          height: positiveNumber(resolution.height, "resolution height"),
        };
      }
      if (variant.fps !== undefined) {
        parsed.fps = positiveNumber(variant.fps, `variant ${index} fps`);
      }
      if (variant.codec !== undefined) {
        parsed.codec = parseCodec(variant.codec);
      }
      if (variant.bitrateKbps !== undefined) {
        parsed.bitrateKbps = positiveNumber(
          variant.bitrateKbps,
          `variant ${index} bitrateKbps`,
        );
      }

      return parsed;
    }),
    supportsCodecSelection: input.supportsCodecSelection,
  };
}
