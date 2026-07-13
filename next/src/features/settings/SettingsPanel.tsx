import { useState, type FormEvent } from "react";

import type {
  LegacyMigrationBackend,
  LegacyMigrationPreview,
  LegacyStorageSnapshot,
  SettingsBackend,
} from "../../api/backend";
import type { Settings } from "../../domain/settings";

const qualityNames: Record<Settings["quality"]["preference"], string> = {
  best: "Best",
  worst: "Worst",
  audioOnly: "Audio only",
};

const themeNames: Record<Settings["theme"], string> = {
  system: "System",
  dark: "Dark",
  light: "Light",
};

export function SettingsPanel({
  backend,
  settings: initialSettings,
  onSaved,
}: {
  backend: SettingsBackend & LegacyMigrationBackend;
  settings: Settings;
  onSaved: (settings: Settings) => void;
}) {
  const [settings, setSettings] = useState(initialSettings);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [migration, setMigration] = useState<LegacyMigrationPreview>();
  const [legacySnapshot, setLegacySnapshot] = useState<LegacyStorageSnapshot>();
  const [migrationBusy, setMigrationBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError("");
    setMessage("Saving settings...");
    try {
      const saved = await backend.saveSettings(settings);
      setSettings(saved);
      onSaved(saved);
      setMessage("Settings saved");
    } catch (reason) {
      setMessage("");
      setError(
        reason instanceof Error ? reason.message : "Could not save settings",
      );
    }
  }

  async function selectLegacyExport(file: File | undefined) {
    setError("");
    setMessage("");
    setMigration(undefined);
    setLegacySnapshot(undefined);
    if (!file) return;

    try {
      const parsed: unknown = JSON.parse(await file.text());
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw new Error("Legacy export must be a JSON object");
      }
      const allowed = new Set([
        "settings",
        "channelsettings",
        "auth",
        "search",
        "window",
        "versioncheck",
        "app",
      ]);
      const snapshot: LegacyStorageSnapshot = {};
      for (const [key, value] of Object.entries(parsed)) {
        if (allowed.has(key) && typeof value === "string") {
          Object.assign(snapshot, { [key]: value });
        }
      }
      if (Object.keys(snapshot).length === 0) {
        throw new Error(
          "No supported legacy namespaces were found in this file",
        );
      }
      setLegacySnapshot(snapshot);
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not read legacy export",
      );
    }
  }

  async function previewMigration() {
    if (!legacySnapshot) return;
    setError("");
    setMessage("");
    setMigrationBusy(true);
    try {
      setMigration(await backend.previewLegacyMigration(legacySnapshot));
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not preview legacy settings",
      );
    } finally {
      setMigrationBusy(false);
    }
  }

  async function importMigration() {
    if (!legacySnapshot) return;
    setError("");
    setMessage("");
    setMigrationBusy(true);
    try {
      const imported = await backend.confirmLegacyMigration(legacySnapshot);
      setMigration(imported);
      setSettings(imported.settings);
      onSaved(imported.settings);
      setMessage("Legacy settings imported");
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not import legacy settings",
      );
    } finally {
      setMigrationBusy(false);
    }
  }

  const safeMigrationRows =
    migration?.status === "ready"
      ? [
          [
            "Player executable",
            settings.player.path ?? "Streamlink default",
            migration.settings.player.path ?? "Streamlink default",
          ],
          [
            "Player arguments",
            settings.player.arguments.join("\n") || "None",
            migration.settings.player.arguments.join("\n") || "None",
          ],
          [
            "Quality preference",
            qualityNames[settings.quality.preference],
            qualityNames[migration.settings.quality.preference],
          ],
          [
            "Maximum video height",
            settings.quality.maximumHeight
              ? `${settings.quality.maximumHeight}p`
              : "No limit",
            migration.settings.quality.maximumHeight
              ? `${migration.settings.quality.maximumHeight}p`
              : "No limit",
          ],
          [
            "Maximum frame rate",
            settings.quality.maximumFps
              ? `${settings.quality.maximumFps} fps`
              : "No limit",
            migration.settings.quality.maximumFps
              ? `${migration.settings.quality.maximumFps} fps`
              : "No limit",
          ],
          ["Language", settings.language, migration.settings.language],
          [
            "Theme",
            themeNames[settings.theme],
            themeNames[migration.settings.theme],
          ],
          [
            "Live channel notifications",
            settings.notifications.liveChannels ? "Enabled" : "Disabled",
            migration.settings.notifications.liveChannels
              ? "Enabled"
              : "Disabled",
          ],
        ]
      : [];

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
        <fieldset className="migration-preview">
          <legend>Legacy settings import</legend>
          <p>
            This app cannot access the separate NW.js Chromium profile
            automatically. Select a namespace JSON export from the previous app
            to preview it. The file remains unchanged, and plaintext credentials
            are always skipped.
          </p>
          <label>
            Legacy namespace export
            <input
              type="file"
              accept="application/json,.json"
              onChange={(event) =>
                void selectLegacyExport(event.currentTarget.files?.[0])
              }
            />
          </label>
          <button
            type="button"
            disabled={migrationBusy || !legacySnapshot}
            onClick={previewMigration}
          >
            Preview legacy import
          </button>
          {migration?.status === "noData" ? (
            <p>No legacy settings were found.</p>
          ) : null}
          {migration?.status === "alreadyCompleted" ? (
            <p>Legacy settings were already imported.</p>
          ) : null}
          {migration && migration.changes.length > 0 ? (
            <ul className="migration-changes">
              {migration.changes.map((change) =>
                change.outcome === "skippedSensitive" ? (
                  <li key={`${change.field}-${change.outcome}`}>
                    <strong>Sensitive fields</strong>: skipped. Plaintext OAuth
                    credentials are never imported.
                  </li>
                ) : (
                  <li key={`${change.field}-${change.outcome}`}>
                    <strong>{change.field}</strong>: {change.outcome}.{" "}
                    {change.detail}
                  </li>
                ),
              )}
            </ul>
          ) : null}
          {migration?.channels.map((channel) => (
            <section key={channel.channelId}>
              <h3>Channel {channel.channelId}</h3>
              <ul className="migration-changes">
                {channel.preferences.map((preference) => (
                  <li key={preference.field}>
                    <strong>{preference.field}</strong>: {preference.outcome}.{" "}
                    {preference.detail}
                  </li>
                ))}
              </ul>
            </section>
          ))}
          {safeMigrationRows.length > 0 ? (
            <>
              <table className="migration-values">
                <caption>Current and proposed safe settings</caption>
                <thead>
                  <tr>
                    <th scope="col">Setting</th>
                    <th scope="col">Current</th>
                    <th scope="col">Proposed</th>
                  </tr>
                </thead>
                <tbody>
                  {safeMigrationRows.map(([label, current, proposed]) => (
                    <tr key={label}>
                      <th scope="row">{label}</th>
                      <td>{current}</td>
                      <td>{proposed}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <button
                type="button"
                disabled={migrationBusy}
                onClick={importMigration}
              >
                Import supported settings
              </button>
            </>
          ) : null}
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
