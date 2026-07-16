import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { BrowserBackend, defaultSettings } from "../../api/backend";
import { SettingsPanel } from "./SettingsPanel";

describe("SettingsPanel", () => {
  it("edits and persists non-secret desktop settings", async () => {
    const saveSettings = vi.fn(async (settings) => settings);
    const backend = new BrowserBackend({
      loadSettings: async () => defaultSettings,
      saveSettings,
    });
    render(
      <SettingsPanel
        backend={backend}
        settings={defaultSettings}
        onSaved={() => undefined}
      />,
    );

    expect(
      screen.queryByLabelText("Notify when followed channels go live"),
    ).not.toBeInTheDocument();

    fireEvent.change(await screen.findByLabelText("Streamlink executable"), {
      target: { value: "/usr/bin/streamlink" },
    });
    fireEvent.change(screen.getByLabelText("Maximum video height"), {
      target: { value: "1440" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    await waitFor(() => expect(saveSettings).toHaveBeenCalledOnce());
    expect(saveSettings.mock.calls[0]![0]).toMatchObject({
      schemaVersion: 1,
      streamlinkPath: "/usr/bin/streamlink",
      quality: { maximumHeight: 1440 },
    });
    expect(screen.getByRole("status")).toHaveTextContent("Settings saved");
  });

  it("probes and explains the separate playback prerequisites", async () => {
    const getStreamlinkStatus = vi.fn(async () => ({
      source: "path" as const,
      version: { major: 8, minor: 4, patch: 0 },
      compatibility: "supported" as const,
    }));
    const getPlayerStatus = vi.fn(async () => ({
      state: "unconfigured" as const,
    }));
    const backend = new BrowserBackend({
      getStreamlinkStatus,
      getPlayerStatus,
    });

    render(
      <SettingsPanel
        backend={backend}
        settings={defaultSettings}
        onSaved={() => undefined}
      />,
    );

    expect(
      screen.getByText(/Streamlink 8.x and a compatible external player/),
    ).toHaveTextContent("not bundled");
    expect(await screen.findByText("Detected Streamlink 8.4.0")).toBeVisible();
    expect(screen.getByText(/Player is not configured/)).toBeVisible();
    expect(
      screen.getByText(
        /default player discovery runs only when playback starts/,
      ),
    ).toBeVisible();
    expect(getStreamlinkStatus).toHaveBeenCalledOnce();
    expect(getPlayerStatus).toHaveBeenCalledOnce();
  });

  it("reports a configured player only when its executable is available", async () => {
    const settings = {
      ...defaultSettings,
      player: { path: "/usr/bin/mpv", arguments: [] },
    };
    const backend = new BrowserBackend({
      getPlayerStatus: async () => ({ state: "configuredAvailable" }),
    });

    render(
      <SettingsPanel
        backend={backend}
        settings={settings}
        onSaved={() => undefined}
      />,
    );

    expect(
      await screen.findByText("Configured player is available."),
    ).toBeVisible();
    expect(screen.getByText("/usr/bin/mpv")).toBeVisible();
  });

  it("reports when a configured player executable has disappeared", async () => {
    const settings = {
      ...defaultSettings,
      player: { path: "C:\\Tools\\mpv.exe", arguments: [] },
    };
    const backend = new BrowserBackend({
      getPlayerStatus: async () => ({ state: "configuredMissing" }),
    });

    render(
      <SettingsPanel
        backend={backend}
        settings={settings}
        onSaved={() => undefined}
      />,
    );

    expect(
      await screen.findByText("Configured player is missing."),
    ).toBeVisible();
    expect(screen.getByText("C:\\Tools\\mpv.exe")).toBeVisible();
    expect(screen.getByText(/Choose an existing executable/)).toBeVisible();
  });

  it("reports a missing Streamlink probe without claiming detection", async () => {
    const backend = new BrowserBackend({
      getStreamlinkStatus: async () => {
        throw new Error("Streamlink was not found");
      },
    });

    render(
      <SettingsPanel
        backend={backend}
        settings={defaultSettings}
        onSaved={() => undefined}
      />,
    );

    expect(
      await screen.findByText("Streamlink was not detected."),
    ).toBeVisible();
    expect(screen.getByText("Streamlink was not found")).toBeVisible();
    expect(screen.queryByText(/Detected Streamlink/)).not.toBeInTheDocument();
  });

  it("does not report an unsaved player path as configured", async () => {
    const backend = new BrowserBackend();
    render(
      <SettingsPanel
        backend={backend}
        settings={defaultSettings}
        onSaved={() => undefined}
      />,
    );

    expect(await screen.findByText("Player is not configured.")).toBeVisible();

    fireEvent.change(screen.getByLabelText("Player executable"), {
      target: { value: "/not-yet-validated/player" },
    });

    expect(screen.getByText("Player is not configured.")).toBeVisible();
    expect(
      screen.queryByText("Configured player is available."),
    ).not.toBeInTheDocument();
  });

  it("previews legacy settings and imports only after explicit confirmation", async () => {
    const currentSettings = {
      ...defaultSettings,
      player: { path: "/usr/bin/vlc", arguments: ["--quiet"] },
      quality: {
        preference: "best" as const,
        maximumHeight: 1080,
        maximumFps: 60,
      },
      notifications: { ...defaultSettings.notifications, liveChannels: false },
    };
    const proposedSettings = {
      ...currentSettings,
      player: {
        path: "/usr/bin/mpv",
        arguments: ["--fullscreen", "--no-osc"],
      },
      quality: {
        preference: "worst" as const,
        maximumHeight: 720,
        maximumFps: 30,
      },
      theme: "dark" as const,
      language: "de",
      notifications: { ...currentSettings.notifications, liveChannels: true },
    };
    const previewLegacyMigration = vi.fn(async () => ({
      status: "ready" as const,
      settings: proposedSettings,
      changes: [
        {
          field: "gui.theme",
          outcome: "imported" as const,
          detail: "Mapped to the typed settings model",
        },
        {
          field: "auth.access_token",
          outcome: "skippedSensitive" as const,
          detail: "Plaintext OAuth credentials are never imported",
        },
      ],
      channels: [
        {
          channelId: "42",
          preferences: [
            {
              field: "streaming_quality",
              outcome: "unsupported" as const,
              detail: "Per-channel overrides are not supported",
            },
          ],
        },
      ],
    }));
    const confirmLegacyMigration = vi.fn(async () => ({
      ...(await previewLegacyMigration()),
      status: "completed" as const,
    }));
    const onSaved = vi.fn();
    const backend = new BrowserBackend({
      previewLegacyMigration,
      confirmLegacyMigration,
    });
    render(
      <SettingsPanel
        backend={backend}
        settings={currentSettings}
        onSaved={onSaved}
      />,
    );

    const legacyExport = new File(
      [
        '{"settings":"{\\"settings\\":{\\"records\\":{\\"raw\\":\\"raw-local-storage-record\\"}}}","auth":"{\\"access_token\\":\\"component-oauth-token\\",\\"authorization\\":\\"Bearer component-authorization\\",\\"client_secret\\":\\"component-api-credential\\"}"}',
      ],
      "legacy-settings.json",
      { type: "application/json" },
    );
    Object.defineProperty(legacyExport, "text", {
      value: async () =>
        '{"settings":"{\\"settings\\":{\\"records\\":{\\"raw\\":\\"raw-local-storage-record\\"}}}","auth":"{\\"access_token\\":\\"component-oauth-token\\",\\"authorization\\":\\"Bearer component-authorization\\",\\"client_secret\\":\\"component-api-credential\\"}"}',
    });
    fireEvent.change(screen.getByLabelText("Legacy namespace export"), {
      target: { files: [legacyExport] },
    });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Preview legacy import" }),
      ).toBeEnabled(),
    );
    expect(
      screen.queryByRole("button", { name: "Import supported settings" }),
    ).not.toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "Preview legacy import" }),
    );
    const values = await screen.findByRole("table", {
      name: "Current and proposed safe settings",
    });
    expect(
      within(values).getByRole("columnheader", { name: "Current" }),
    ).toBeInTheDocument();
    expect(
      within(values).getByRole("columnheader", { name: "Proposed" }),
    ).toBeInTheDocument();
    for (const rowName of [
      "Player executable /usr/bin/vlc /usr/bin/mpv",
      /^Player arguments --quiet --fullscreen\s+--no-osc$/,
      "Quality preference Best Worst",
      "Maximum video height 1080p 720p",
      "Maximum frame rate 60 fps 30 fps",
      "Language en de",
      "Theme System Dark",
      "Live channel notifications Disabled Enabled",
    ]) {
      expect(within(values).getByRole("row", { name: rowName })).toBeVisible();
    }
    expect(await screen.findByText("gui.theme")).toBeInTheDocument();
    expect(
      screen.getByText(/OAuth credentials are never imported/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Channel 42/)).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("component-oauth-token");
    expect(document.body).not.toHaveTextContent("component-authorization");
    expect(document.body).not.toHaveTextContent("component-api-credential");
    expect(document.body).not.toHaveTextContent("raw-local-storage-record");
    expect(confirmLegacyMigration).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: "Import supported settings" }),
    );
    await waitFor(() => expect(confirmLegacyMigration).toHaveBeenCalledOnce());
    expect(previewLegacyMigration).toHaveBeenCalledWith({
      settings: '{"settings":{"records":{"raw":"raw-local-storage-record"}}}',
      auth: '{"access_token":"component-oauth-token","authorization":"Bearer component-authorization","client_secret":"component-api-credential"}',
    });
    expect(confirmLegacyMigration).toHaveBeenCalledWith({
      settings: '{"settings":{"records":{"raw":"raw-local-storage-record"}}}',
      auth: '{"access_token":"component-oauth-token","authorization":"Bearer component-authorization","client_secret":"component-api-credential"}',
    });
    expect(onSaved).toHaveBeenCalledWith(
      expect.objectContaining({ theme: "dark", language: "de" }),
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Legacy settings imported",
    );
  });
});
