import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

async function mockDesktopBoundary(page: Page) {
  await page.addInitScript(() => {
    const settings = {
      schemaVersion: 1,
      player: { arguments: [] },
      codecPreference: { allowed: ["h264", "h265", "av1"] },
      quality: { preference: "best" },
      theme: "system",
      language: "en",
      notifications: { liveChannels: true, playbackErrors: true },
      hotkey: { enabled: false, accelerator: "Ctrl+Shift+S" },
    };
    const stream = {
      id: "stream-1",
      userId: "user-1",
      userLogin: "signalnoise",
      userName: "Signal Noise",
      gameId: "game-1",
      gameName: "Science & Technology",
      title: "Building a tiny synthesizer",
      viewerCount: 1842,
      startedAt: "2026-07-12T10:00:00Z",
      thumbnailUrl:
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_signalnoise-640x360.jpg",
      isMature: false,
    };
    const channel = {
      id: "user-1",
      broadcasterLogin: "signalnoise",
      displayName: "Signal Noise",
      broadcasterLanguage: "en",
      title: "Building a tiny synthesizer",
      gameName: "Science & Technology",
      thumbnailUrl:
        "https://static-cdn.jtvnw.net/jtv_user_pictures/signalnoise.png",
      isLive: true,
    };
    const authenticatedSession = {
      status: "authenticated",
      user: {
        id: "user-1",
        login: "signalnoise",
        displayName: "Signal Noise",
        profileImageUrl: channel.thumbnailUrl,
      },
      expiresAt: "2026-07-12T12:00:00Z",
    };
    let session: { status: string; user?: unknown; expiresAt?: string } =
      new URLSearchParams(window.location.search).get("e2eSession") ===
      "anonymous"
        ? { status: "anonymous" }
        : authenticatedSession;
    const calls: Array<{ command: string; args: unknown }> = [];
    Object.assign(window, { __e2eCalls: calls });
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (command: string, args: Record<string, unknown> = {}) => {
          calls.push({ command, args });
          switch (command) {
            case "get_settings":
              return settings;
            case "save_settings":
              return args.settings;
            case "get_streamlink_status":
              return {
                source: "path",
                version: { major: 8, minor: 4, patch: 0 },
                compatibility: "supported",
              };
            case "get_player_status":
              return { state: "unconfigured" };
            case "get_twitch_session":
              return session;
            case "begin_twitch_login":
              return {
                verificationUri: "https://www.twitch.tv/activate",
                userCode: "E2E-CODE",
                expiresInSeconds: 60,
                pollingIntervalSeconds: 1,
              };
            case "plugin:opener|open_url":
              return null;
            case "poll_twitch_login":
              session = authenticatedSession;
              return session;
            case "sign_out_twitch":
              session = { status: "anonymous" };
              return null;
            case "twitch_streams":
            case "twitch_followed_streams":
              return { items: [stream] };
            case "twitch_followed_channels":
              return {
                items: [
                  {
                    broadcasterId: "user-1",
                    broadcasterLogin: "signalnoise",
                    broadcasterName: "Signal Noise",
                    followedAt: "2026-07-01T10:00:00Z",
                  },
                ],
              };
            case "twitch_top_games":
            case "twitch_search_categories":
              return {
                items: [
                  {
                    id: "game-1",
                    name: "Science & Technology",
                    boxArtUrl:
                      "https://static-cdn.jtvnw.net/ttv-boxart/509670-285x380.jpg",
                  },
                ],
              };
            case "twitch_search_channels":
              return { items: [channel] };
            case "inspect_streams":
              return {
                supportsCodecSelection: true,
                variants: [
                  {
                    name: "1080p60",
                    resolution: { width: 1920, height: 1080 },
                    fps: 60,
                    codec: "h264",
                    aliases: ["best"],
                  },
                  {
                    name: "1440p60_hevc",
                    resolution: { width: 2560, height: 1440 },
                    fps: 60,
                    codec: "h265",
                    aliases: [],
                  },
                ],
              };
            case "launch_stream":
              return { status: "running", diagnostics: [] };
            case "stop_stream":
              return { status: "stopped", diagnostics: [] };
            case "preview_legacy_migration":
              return {
                status: "ready",
                settings: {
                  ...settings,
                  player: {
                    path: "/usr/bin/mpv",
                    arguments: ["--fullscreen", "--no-osc"],
                  },
                  quality: {
                    preference: "worst",
                    maximumHeight: 720,
                    maximumFps: 30,
                  },
                  theme: "dark",
                  language: "de",
                  notifications: {
                    ...settings.notifications,
                    liveChannels: false,
                  },
                },
                changes: [
                  {
                    field: "gui.theme",
                    outcome: "imported",
                    detail: "Mapped to the typed settings model",
                  },
                  {
                    field: "auth.access_token",
                    outcome: "skippedSensitive",
                    detail: "Plaintext OAuth credentials are never imported",
                  },
                ],
                channels: [],
              };
            case "confirm_legacy_migration":
              return {
                status: "completed",
                settings: {
                  ...settings,
                  player: {
                    path: "/usr/bin/mpv",
                    arguments: ["--fullscreen", "--no-osc"],
                  },
                  quality: {
                    preference: "worst",
                    maximumHeight: 720,
                    maximumFps: 30,
                  },
                  theme: "dark",
                  language: "de",
                  notifications: {
                    ...settings.notifications,
                    liveChannels: false,
                  },
                },
                changes: [],
                channels: [],
              };
            default:
              throw new Error(`Unexpected Tauri command: ${command}`);
          }
        },
      },
    });
  });
}

test.beforeEach(async ({ page }) => {
  await mockDesktopBoundary(page);
  await page.goto("/");
});

test("moves from anonymous device login through detection and playback", async ({
  page,
}) => {
  await page.goto("/?e2eSession=anonymous");
  await expect(page.getByText("Sign in with Twitch to browse")).toBeVisible();
  await page.getByRole("button", { name: "Sign in with Twitch" }).click();
  await expect(page.getByText("E2E-CODE")).toBeVisible();
  await expect(page.getByRole("status")).toHaveText(
    "Waiting for Twitch authorization...",
  );

  await expect(page.getByRole("heading", { name: "Live now" })).toBeVisible();
  await expect(page.getByText("Building a tiny synthesizer")).toBeVisible();
  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByText("Detected Streamlink 8.4.0")).toBeVisible();

  await page.getByRole("button", { name: "Search" }).click();
  await page.getByRole("searchbox", { name: "Channel name" }).fill("signal");
  await page
    .getByRole("search")
    .getByRole("button", { name: "Search", exact: true })
    .click();
  await page.getByRole("button", { name: /Signal Noise/ }).click();
  await expect(
    page.getByRole("heading", { name: "Choose the broadcast signal" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Launch stream" }).click();
  await expect(page.getByText("Playing externally")).toBeVisible();
  await page.getByRole("button", { name: "Stop playback" }).click();
  await expect(
    page.getByRole("button", { name: "Launch stream" }),
  ).toBeVisible();

  const commands = await page.evaluate(() =>
    (
      window as typeof window & {
        __e2eCalls: Array<{ command: string }>;
      }
    ).__e2eCalls.map((call) => call.command),
  );
  for (const command of [
    "get_twitch_session",
    "begin_twitch_login",
    "plugin:opener|open_url",
    "poll_twitch_login",
    "twitch_streams",
    "get_streamlink_status",
    "twitch_search_channels",
    "inspect_streams",
    "launch_stream",
    "stop_stream",
  ]) {
    expect(commands).toContain(command);
  }
});

test("meets WCAG 2 A and AA on the primary route", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Live now" })).toBeVisible();

  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();

  expect(results.violations).toEqual([]);
});

test("browses public and followed content", async ({ page }) => {
  await expect(page.getByRole("heading", { name: "Live now" })).toBeVisible();
  await expect(page.getByText("Building a tiny synthesizer")).toBeVisible();

  await page.getByRole("button", { name: "Categories" }).click();
  await expect(
    page.getByRole("heading", { name: "Top categories" }),
  ).toBeVisible();
  await expect(page.getByText("Science & Technology")).toBeVisible();

  await page.getByRole("button", { name: "Following" }).click();
  await expect(
    page.getByRole("heading", { name: "Following live" }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: /Signal Noise/ })).toBeVisible();

  await page.getByRole("button", { name: "Channels" }).click();
  await expect(
    page.getByRole("heading", { name: "Followed channels" }),
  ).toBeVisible();
  await expect(page.getByText("@signalnoise")).toBeVisible();
});

test("searches, inspects, launches, changes quality, and stops", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Search" }).click();
  await page.getByRole("searchbox", { name: "Channel name" }).fill("signal");
  await page
    .getByRole("search")
    .getByRole("button", { name: "Search", exact: true })
    .click();
  await page.getByRole("button", { name: /Signal Noise/ }).click();

  await expect(
    page.getByRole("heading", { name: "Signal Noise", level: 1 }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Choose the broadcast signal" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Launch stream" }).click();
  await expect(page.getByText("Playing externally")).toBeVisible();

  await page.getByRole("radio", { name: /HEVC.*60 fps/ }).click();
  await expect(page.getByRole("note")).toContainText("HEVC needs support");
  await page.getByRole("button", { name: "Stop playback" }).click();
  await expect(
    page.getByRole("button", { name: "Launch stream" }),
  ).toBeVisible();

  const commands = await page.evaluate(() =>
    (
      window as typeof window & {
        __e2eCalls: Array<{ command: string }>;
      }
    ).__e2eCalls.map((call) => call.command),
  );
  expect(
    commands.filter((command) => command === "launch_stream"),
  ).toHaveLength(2);
  expect(commands.filter((command) => command === "stop_stream")).toHaveLength(
    2,
  );
});

test("previews and explicitly confirms legacy settings", async ({ page }) => {
  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByLabel("Legacy namespace export").setInputFiles({
    name: "legacy-settings.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({
        settings: JSON.stringify({ settings: { records: {} } }),
        auth: JSON.stringify({
          access_token: "e2e-oauth-token",
          authorization: "Bearer e2e-authorization",
          client_secret: "e2e-api-credential",
          raw: "e2e-raw-local-storage-record",
        }),
      }),
    ),
  });
  await expect(
    page.getByRole("button", { name: "Import supported settings" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Preview legacy import" }).click();
  const values = page.getByRole("table", {
    name: "Current and proposed safe settings",
  });
  await expect(
    values.getByRole("columnheader", { name: "Current" }),
  ).toBeVisible();
  await expect(
    values.getByRole("columnheader", { name: "Proposed" }),
  ).toBeVisible();
  for (const rowName of [
    "Player executable Streamlink default /usr/bin/mpv",
    "Player arguments None --fullscreen --no-osc",
    "Quality preference Best Worst",
    "Maximum video height No limit 720p",
    "Maximum frame rate No limit 30 fps",
    "Language en de",
    "Theme System Dark",
    "Live channel notifications Enabled Disabled",
  ]) {
    await expect(values.getByRole("row", { name: rowName })).toBeVisible();
  }
  await expect(page.getByText("gui.theme")).toBeVisible();
  await expect(
    page.getByText(/OAuth credentials are never imported/),
  ).toBeVisible();
  await expect(page.locator("body")).not.toContainText("e2e-oauth-token");
  await expect(page.locator("body")).not.toContainText("e2e-authorization");
  await expect(page.locator("body")).not.toContainText("e2e-api-credential");
  await expect(page.locator("body")).not.toContainText(
    "e2e-raw-local-storage-record",
  );

  const before = await page.evaluate(() =>
    (
      window as typeof window & {
        __e2eCalls: Array<{ command: string }>;
      }
    ).__e2eCalls.some((call) => call.command === "confirm_legacy_migration"),
  );
  expect(before).toBe(false);
  await page.getByRole("button", { name: "Import supported settings" }).click();
  await expect(page.getByRole("status")).toHaveText("Legacy settings imported");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).toHaveAttribute("lang", "de");
});
