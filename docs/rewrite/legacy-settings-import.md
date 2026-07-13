# Importing legacy settings

The Tauri application cannot automatically read settings from the previous NW.js application. They use separate webview profiles and origins. The import screen accepts an explicit JSON export, previews every supported change, and writes new settings only after confirmation.

## Legacy profile location

The old manifest name is `streamlink-twitch-gui`. NW.js 0.83 documents `nw.App.dataPath` at these locations:

| Platform | Legacy application data path                                  |
| -------- | ------------------------------------------------------------- |
| Windows  | `%LOCALAPPDATA%\streamlink-twitch-gui`                        |
| Linux    | `~/.config/streamlink-twitch-gui`                             |
| macOS    | `~/Library/Application Support/streamlink-twitch-gui/Default` |

Chromium localStorage is normally under `Local Storage/leveldb` below that active profile. Much older NW.js profiles can instead contain SQLite-era `.localstorage` files. Do not select, copy into, or edit either database for this importer. LevelDB is not a stable export format and may be locked or internally inconsistent while the old application is running.

## Export format

Create the export from the running legacy application, where `nw.Window.window.localStorage` refers to the correct origin. The file is a JSON object whose values are the original namespace strings:

```json
{
  "settings": "{\"settings\":{\"records\":{...}}}",
  "channelsettings": "{\"channel-settings\":{\"records\":{...}}}"
}
```

An export utility or legacy debug console can generate the object without changing storage:

```js
const storage = nw.Window.get().window.localStorage;
const names = [
  "settings",
  "channelsettings",
  "auth",
  "search",
  "window",
  "versioncheck",
  "app",
];
const exported = Object.fromEntries(
  names.flatMap((name) => {
    const value = storage.getItem(name);
    return value === null ? [] : [[name, value]];
  }),
);
const link = document.createElement("a");
link.href = URL.createObjectURL(
  new Blob([JSON.stringify(exported, null, 2)], { type: "application/json" }),
);
link.download = "streamlink-twitch-gui-legacy-settings.json";
link.click();
```

The export may contain the legacy plaintext OAuth namespace. The importer detects but never imports or displays token values. Keep the export private and delete it after migration if it contains `auth`.

## Import

1. Open Settings in the new application.
2. Select the exported JSON file under **Legacy settings import**.
3. Choose **Preview legacy import** and review imported, unsupported, invalid, and sensitive fields.
4. Choose **Import supported settings** only if the preview is correct.

The selected file and legacy profile are read-only. Import creates only the new typed settings file and a one-time completion marker.
