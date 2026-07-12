import { useEffect, useState, type FormEvent } from "react";

import type { SettingsBackend } from "../../api/backend";
import type { Settings } from "../../domain/settings";

export function SettingsPanel({ backend }: { backend: SettingsBackend }) {
  const [settings, setSettings] = useState<Settings>();
  const [message, setMessage] = useState("Loading settings...");
  const [error, setError] = useState("");

  useEffect(() => {
    let current = true;
    void backend.loadSettings().then(
      (loaded) => {
        if (current) {
          setSettings(loaded);
          setMessage("");
        }
      },
      (reason: unknown) =>
        current &&
        setError(
          reason instanceof Error ? reason.message : "Could not load settings",
        ),
    );
    return () => {
      current = false;
    };
  }, [backend]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!settings) return;
    setError("");
    setMessage("Saving settings...");
    try {
      setSettings(await backend.saveSettings(settings));
      setMessage("Settings saved");
    } catch (reason) {
      setMessage("");
      setError(
        reason instanceof Error ? reason.message : "Could not save settings",
      );
    }
  }

  if (!settings)
    return (
      <p role="status" className="state-panel">
        {error || message}
      </p>
    );
  return (
    <>
      <header className="route-header">
        <p className="eyebrow">Control room</p>
        <h1>Settings</h1>
        <p>Playback tools, preferred formats, and desktop behavior.</p>
      </header>
      <form className="settings-form" onSubmit={submit}>
        <fieldset>
          <legend>Executables</legend>
          <label>
            Streamlink executable
            <input
              value={settings.streamlinkPath ?? ""}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  streamlinkPath: event.target.value || undefined,
                })
              }
              placeholder="Auto-detect from PATH"
            />
          </label>
          <label>
            Player executable
            <input
              value={settings.player.path ?? ""}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  player: {
                    ...settings.player,
                    path: event.target.value || undefined,
                  },
                })
              }
              placeholder="Streamlink default player"
            />
          </label>
          <label>
            Player arguments
            <textarea
              value={settings.player.arguments.join("\n")}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  player: {
                    ...settings.player,
                    arguments: event.target.value.split("\n").filter(Boolean),
                  },
                })
              }
              placeholder="One argument per line"
            />
          </label>
        </fieldset>
        <fieldset>
          <legend>Playback preference</legend>
          <label>
            Preferred codec
            <select
              value={settings.codecPreference.preferred ?? "auto"}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  codecPreference: {
                    ...settings.codecPreference,
                    preferred:
                      event.target.value === "auto"
                        ? undefined
                        : (event.target
                            .value as Settings["codecPreference"]["preferred"]),
                  },
                })
              }
            >
              <option value="auto">Best available</option>
              <option value="h264">H.264</option>
              <option value="h265">HEVC</option>
              <option value="av1">AV1</option>
            </select>
          </label>
          <label>
            Maximum video height
            <input
              type="number"
              min="144"
              step="1"
              value={settings.quality.maximumHeight ?? ""}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  quality: {
                    ...settings.quality,
                    maximumHeight: event.target.value
                      ? Number(event.target.value)
                      : undefined,
                  },
                })
              }
              placeholder="No limit"
            />
          </label>
        </fieldset>
        <fieldset>
          <legend>Appearance and desktop</legend>
          <label>
            Theme
            <select
              value={settings.theme}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  theme: event.target.value as Settings["theme"],
                })
              }
            >
              <option value="system">System</option>
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </label>
          <label>
            Language
            <input
              value={settings.language}
              onChange={(event) =>
                setSettings({ ...settings, language: event.target.value })
              }
            />
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={settings.notifications.liveChannels}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  notifications: {
                    ...settings.notifications,
                    liveChannels: event.target.checked,
                  },
                })
              }
            />
            Notify when followed channels go live
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={settings.notifications.playbackErrors}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  notifications: {
                    ...settings.notifications,
                    playbackErrors: event.target.checked,
                  },
                })
              }
            />
            Notify about playback errors
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={settings.hotkey.enabled}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  hotkey: { ...settings.hotkey, enabled: event.target.checked },
                })
              }
            />
            Enable global play or stop hotkey
          </label>
          <label>
            Global hotkey
            <input
              value={settings.hotkey.accelerator}
              onChange={(event) =>
                setSettings({
                  ...settings,
                  hotkey: {
                    ...settings.hotkey,
                    accelerator: event.target.value,
                  },
                })
              }
            />
          </label>
        </fieldset>
        {error ? <p role="alert">{error}</p> : null}
        {message ? <p role="status">{message}</p> : null}
        <button className="primary-action" type="submit">
          Save settings
        </button>
      </form>
    </>
  );
}
