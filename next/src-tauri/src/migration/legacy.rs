use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    domain::stream::{QualityPreference, Settings, Theme},
    settings::{SettingsError, SettingsStore},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyStorageSnapshot {
    #[serde(default)]
    pub settings: Option<String>,
    #[serde(default)]
    pub channelsettings: Option<String>,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub window: Option<String>,
    #[serde(default)]
    pub versioncheck: Option<String>,
    #[serde(default)]
    pub app: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LegacyMigrationStatus {
    Ready,
    NoData,
    Completed,
    AlreadyCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MigrationOutcome {
    Imported,
    Unsupported,
    SkippedSensitive,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationChange {
    pub field: String,
    pub outcome: MigrationOutcome,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyChannelPreview {
    pub channel_id: String,
    pub preferences: Vec<MigrationChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyMigrationPreview {
    pub status: LegacyMigrationStatus,
    pub settings: Settings,
    pub changes: Vec<MigrationChange>,
    pub channels: Vec<LegacyChannelPreview>,
}

#[derive(Debug, thiserror::Error)]
pub enum LegacyMigrationError {
    #[error("legacy migration requires explicit confirmation")]
    ConfirmationRequired,
    #[error(transparent)]
    Settings(#[from] SettingsError),
    #[error("legacy migration marker could not be written: {0}")]
    Marker(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct LegacyMigrator {
    settings_store: SettingsStore,
    marker_path: PathBuf,
}

impl LegacyMigrator {
    pub fn new(settings_store: SettingsStore, marker_path: PathBuf) -> Self {
        Self {
            settings_store,
            marker_path,
        }
    }

    pub fn preview_current(
        &self,
        snapshot: &LegacyStorageSnapshot,
    ) -> Result<LegacyMigrationPreview, LegacyMigrationError> {
        Ok(Self::preview(
            snapshot,
            self.settings_store.load()?,
            self.marker_path.exists(),
        ))
    }

    pub fn preview(
        snapshot: &LegacyStorageSnapshot,
        current: Settings,
        completed: bool,
    ) -> LegacyMigrationPreview {
        let mut preview = LegacyMigrationPreview {
            status: if completed {
                LegacyMigrationStatus::AlreadyCompleted
            } else if snapshot_is_empty(snapshot) {
                LegacyMigrationStatus::NoData
            } else {
                LegacyMigrationStatus::Ready
            },
            settings: current,
            changes: Vec::new(),
            channels: Vec::new(),
        };
        if completed {
            return preview;
        }

        let aggregate = parse_namespace(snapshot.app.as_deref(), "app", &mut preview.changes);
        let settings_raw = snapshot
            .settings
            .as_deref()
            .map(str::to_owned)
            .or_else(|| aggregate.as_ref()?.get("settings").map(Value::to_string));
        if let Some(root) =
            parse_namespace(settings_raw.as_deref(), "settings", &mut preview.changes)
        {
            map_settings(&root, &mut preview);
        }

        let channels_raw = snapshot
            .channelsettings
            .as_deref()
            .map(str::to_owned)
            .or_else(|| {
                aggregate
                    .as_ref()?
                    .get("channelsettings")
                    .map(Value::to_string)
            });
        if let Some(root) = parse_namespace(
            channels_raw.as_deref(),
            "channelsettings",
            &mut preview.changes,
        ) {
            map_channels(&root, &mut preview);
        }

        if let Some(auth) = parse_namespace(snapshot.auth.as_deref(), "auth", &mut preview.changes)
            && records(&auth, "auth")
                .and_then(|items| items.values().next())
                .and_then(|record| record.get("access_token"))
                .is_some_and(|token| !token.is_null())
        {
            change(
                &mut preview.changes,
                "auth.access_token",
                MigrationOutcome::SkippedSensitive,
                "Plaintext OAuth credentials are never imported",
            );
        }
        preview
    }

    pub fn import(
        &self,
        snapshot: &LegacyStorageSnapshot,
        confirmed: bool,
    ) -> Result<LegacyMigrationPreview, LegacyMigrationError> {
        if !confirmed {
            return Err(LegacyMigrationError::ConfirmationRequired);
        }
        let mut preview = self.preview_current(snapshot)?;
        if preview.status == LegacyMigrationStatus::AlreadyCompleted {
            return Ok(preview);
        }
        self.settings_store.save(&preview.settings)?;
        if let Some(parent) = self.marker_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.marker_path)?;
        marker.write_all(b"completed\n")?;
        marker.sync_all()?;
        preview.status = LegacyMigrationStatus::Completed;
        Ok(preview)
    }
}

fn snapshot_is_empty(snapshot: &LegacyStorageSnapshot) -> bool {
    snapshot.settings.is_none()
        && snapshot.channelsettings.is_none()
        && snapshot.auth.is_none()
        && snapshot.search.is_none()
        && snapshot.window.is_none()
        && snapshot.versioncheck.is_none()
        && snapshot.app.is_none()
}

fn parse_namespace(
    raw: Option<&str>,
    name: &str,
    changes: &mut Vec<MigrationChange>,
) -> Option<Value> {
    let raw = raw?;
    match serde_json::from_str::<Value>(raw) {
        Ok(value) if value.is_object() => Some(value),
        Ok(_) => {
            change(
                changes,
                name,
                MigrationOutcome::Invalid,
                "Legacy namespace is not a JSON object",
            );
            None
        }
        Err(_) => {
            change(
                changes,
                name,
                MigrationOutcome::Invalid,
                "Legacy namespace contains invalid JSON",
            );
            None
        }
    }
}

fn records<'a>(root: &'a Value, model: &str) -> Option<&'a serde_json::Map<String, Value>> {
    root.get(model)?.get("records")?.as_object()
}

fn map_settings(root: &Value, preview: &mut LegacyMigrationPreview) {
    let Some(record) = records(root, "settings").and_then(|items| {
        items
            .get("1")
            .or_else(|| items.values().next())
            .and_then(Value::as_object)
    }) else {
        change(
            &mut preview.changes,
            "settings",
            MigrationOutcome::Invalid,
            "No legacy settings record was found",
        );
        return;
    };

    if let Some(gui) = record.get("gui").and_then(Value::as_object) {
        map_theme(gui.get("theme"), preview);
        map_language(gui.get("language"), preview);
    }
    if let Some(streaming) = record.get("streaming").and_then(Value::as_object) {
        map_quality(streaming.get("quality"), preview);
        map_player(streaming, preview);
        if streaming
            .get("twitch_api_header")
            .is_some_and(|value| !value.is_null())
        {
            change(
                &mut preview.changes,
                "streaming.twitch_api_header",
                MigrationOutcome::SkippedSensitive,
                "Plaintext API authorization values are never imported",
            );
        }
    }
    if let Some(enabled) = record
        .get("notification")
        .and_then(|value| value.get("enabled"))
    {
        if let Some(enabled) = enabled.as_bool() {
            preview.settings.notifications.live_channels = enabled;
            change(
                &mut preview.changes,
                "notification.enabled",
                MigrationOutcome::Imported,
                "Mapped to live-channel notifications",
            );
        } else {
            invalid(&mut preview.changes, "notification.enabled");
        }
    }
}

fn map_theme(value: Option<&Value>, preview: &mut LegacyMigrationPreview) {
    let Some(value) = value else { return };
    preview.settings.theme = match value.as_str() {
        Some("system") => Theme::System,
        Some("dark") => Theme::Dark,
        Some("light") => Theme::Light,
        _ => {
            unsupported(&mut preview.changes, "gui.theme");
            return;
        }
    };
    imported(&mut preview.changes, "gui.theme");
}

fn map_language(value: Option<&Value>, preview: &mut LegacyMigrationPreview) {
    let Some(value) = value else { return };
    match value.as_str() {
        Some("auto") => change(
            &mut preview.changes,
            "gui.language",
            MigrationOutcome::Unsupported,
            "Automatic legacy locale selection has no typed target",
        ),
        Some(language) if !language.is_empty() && !language.chars().any(char::is_control) => {
            preview.settings.language = language.to_owned();
            imported(&mut preview.changes, "gui.language");
        }
        _ => invalid(&mut preview.changes, "gui.language"),
    }
}

fn map_quality(value: Option<&Value>, preview: &mut LegacyMigrationPreview) {
    let Some(value) = value else { return };
    let Some(quality) = value.as_str() else {
        invalid(&mut preview.changes, "streaming.quality");
        return;
    };
    preview.settings.quality.maximum_height = None;
    preview.settings.quality.maximum_fps = None;
    preview.settings.quality.preference = match quality {
        "source" => QualityPreference::Best,
        "audio" => QualityPreference::AudioOnly,
        "1440p60" => {
            preview.settings.quality.maximum_height = Some(1440);
            preview.settings.quality.maximum_fps = Some(60);
            QualityPreference::Best
        }
        "high" => {
            preview.settings.quality.maximum_height = Some(720);
            preview.settings.quality.maximum_fps = Some(30);
            QualityPreference::Best
        }
        "medium" => {
            preview.settings.quality.maximum_height = Some(540);
            preview.settings.quality.maximum_fps = Some(30);
            QualityPreference::Best
        }
        "low" => {
            preview.settings.quality.maximum_height = Some(360);
            preview.settings.quality.maximum_fps = Some(30);
            QualityPreference::Best
        }
        _ => {
            unsupported(&mut preview.changes, "streaming.quality");
            return;
        }
    };
    imported(&mut preview.changes, "streaming.quality");
}

fn map_player(streaming: &serde_json::Map<String, Value>, preview: &mut LegacyMigrationPreview) {
    let Some(profile) = streaming.get("player").and_then(Value::as_str) else {
        return;
    };
    let Some(player) = streaming
        .get("players")
        .and_then(Value::as_object)
        .and_then(|players| players.get(profile))
        .and_then(Value::as_object)
    else {
        unsupported(&mut preview.changes, "streaming.player");
        return;
    };
    if let Some(path) = player.get("exec").and_then(Value::as_str) {
        if path.is_empty() || path.chars().any(char::is_control) || !Path::new(path).is_file() {
            invalid(&mut preview.changes, "streaming.player.exec");
        } else {
            preview.settings.player.path = Some(path.to_owned());
            imported(&mut preview.changes, "streaming.player.exec");
        }
    }
    if let Some(arguments) = player.get("args").and_then(Value::as_str) {
        if arguments.chars().any(char::is_control) {
            invalid(&mut preview.changes, "streaming.player.args");
        } else if let Some(arguments) = shlex::split(arguments) {
            preview.settings.player.arguments = arguments;
            imported(&mut preview.changes, "streaming.player.args");
        } else {
            change(
                &mut preview.changes,
                "streaming.player.args",
                MigrationOutcome::Unsupported,
                "Unbalanced legacy argument quoting cannot be imported safely",
            );
        }
    }
    change(
        &mut preview.changes,
        "streaming.player.profileOptions",
        MigrationOutcome::Unsupported,
        "Legacy player-specific switches have no typed target",
    );
}

fn map_channels(root: &Value, preview: &mut LegacyMigrationPreview) {
    let Some(items) = records(root, "channel-settings") else {
        change(
            &mut preview.changes,
            "channelsettings",
            MigrationOutcome::Invalid,
            "No channel settings records were found",
        );
        return;
    };
    for (id, value) in items {
        let Some(record) = value.as_object() else {
            continue;
        };
        let mut preferences = Vec::new();
        for field in [
            "streaming_quality",
            "streaming_low_latency",
            "streaming_twitch_extra_codecs",
            "streams_chat_open",
            "notification_enabled",
        ] {
            if record.get(field).is_some_and(|value| !value.is_null()) {
                change(
                    &mut preferences,
                    field,
                    MigrationOutcome::Unsupported,
                    "Per-channel overrides are not present in the typed settings model",
                );
            }
        }
        if !preferences.is_empty() {
            preview.channels.push(LegacyChannelPreview {
                channel_id: id.clone(),
                preferences,
            });
        }
    }
}

fn imported(changes: &mut Vec<MigrationChange>, field: &str) {
    change(
        changes,
        field,
        MigrationOutcome::Imported,
        "Mapped to the typed settings model",
    );
}

fn unsupported(changes: &mut Vec<MigrationChange>, field: &str) {
    change(
        changes,
        field,
        MigrationOutcome::Unsupported,
        "No safe typed target is available",
    );
}

fn invalid(changes: &mut Vec<MigrationChange>, field: &str) {
    change(
        changes,
        field,
        MigrationOutcome::Invalid,
        "Legacy value has an unexpected type or format",
    );
}

fn change(
    changes: &mut Vec<MigrationChange>,
    field: &str,
    outcome: MigrationOutcome,
    detail: &str,
) {
    changes.push(MigrationChange {
        field: field.to_owned(),
        outcome,
        detail: detail.to_owned(),
    });
}
