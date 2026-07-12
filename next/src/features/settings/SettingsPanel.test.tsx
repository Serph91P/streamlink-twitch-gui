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
    render(<SettingsPanel backend={backend} />);

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
});
