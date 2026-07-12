import { describe, expect, it } from "vitest";

import fixture from "../../fixtures/domain-contracts.json";
import { parseSettings } from "./settings";
import { parseStreamCapabilities } from "./stream";
import { parseTwitchSession } from "./twitch";

describe("domain contracts", () => {
  it("deserializes dynamic stream variants and codec metadata", () => {
    const capabilities = parseStreamCapabilities(fixture.capabilities);

    expect(capabilities.variants).toEqual([
      {
        name: "1440p60-av1",
        resolution: { width: 2560, height: 1440 },
        fps: 60,
        codec: "av1",
        bitrateKbps: 12000,
        aliases: ["best"],
      },
      {
        name: "audio_only",
        codec: "unknown",
        aliases: ["audio_only", "worst"],
      },
    ]);
  });

  it("round-trips codec preferences, quality constraints, and player settings", () => {
    const settings = parseSettings(fixture.settings);

    expect(JSON.parse(JSON.stringify(settings))).toEqual(fixture.settings);
  });

  it("rejects unknown mutable settings fields", () => {
    expect(() =>
      parseSettings({ ...fixture.settings, arbitraryCommand: "rm -rf /" }),
    ).toThrow(/unknown settings field: arbitraryCommand/i);
    expect(() =>
      parseSettings({
        ...fixture.settings,
        quality: { ...fixture.settings.quality, futureLimit: 1 },
      }),
    ).toThrow(/unknown quality field: futureLimit/i);
  });

  it("allows unknown read-only capability fields", () => {
    expect(parseStreamCapabilities(fixture.capabilities).variants).toHaveLength(
      2,
    );
  });

  it("deserializes a frontend-safe Twitch session", () => {
    expect(parseTwitchSession(fixture.session)).toEqual(fixture.session);
    expect(JSON.stringify(parseTwitchSession(fixture.session))).not.toMatch(
      /accessToken|refreshToken/,
    );
  });
});
