import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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

  it("previews legacy settings and imports only after explicit confirmation", async () => {
    const previewLegacyMigration = vi.fn(async () => ({
      status: "ready" as const,
      settings: { ...defaultSettings, theme: "dark" as const, language: "de" },
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
        settings={defaultSettings}
        onSaved={onSaved}
      />,
    );

    const legacyExport = new File(
      ['{"settings":"{\\"settings\\":{\\"records\\":{}}}"}'],
      "legacy-settings.json",
      { type: "application/json" },
    );
    Object.defineProperty(legacyExport, "text", {
      value: async () => '{"settings":"{\\"settings\\":{\\"records\\":{}}}"}',
    });
    fireEvent.change(screen.getByLabelText("Legacy namespace export"), {
      target: { files: [legacyExport] },
    });
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Preview legacy import" }),
      ).toBeEnabled(),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Preview legacy import" }),
    );
    expect(await screen.findByText("gui.theme")).toBeInTheDocument();
    expect(
      screen.getByText(/OAuth credentials are never imported/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Channel 42/)).toBeInTheDocument();
    expect(confirmLegacyMigration).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: "Import supported settings" }),
    );
    await waitFor(() => expect(confirmLegacyMigration).toHaveBeenCalledOnce());
    expect(previewLegacyMigration).toHaveBeenCalledWith({
      settings: '{"settings":{"records":{}}}',
    });
    expect(confirmLegacyMigration).toHaveBeenCalledWith({
      settings: '{"settings":{"records":{}}}',
    });
    expect(onSaved).toHaveBeenCalledWith(
      expect.objectContaining({ theme: "dark", language: "de" }),
    );
    expect(screen.getByRole("status")).toHaveTextContent(
      "Legacy settings imported",
    );
  });
});
