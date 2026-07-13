use std::{fs, path::PathBuf, time::SystemTime};

use streamlink_twitch_gui_lib::{
    migration::{LegacyMigrationStatus, LegacyMigrator, LegacyStorageSnapshot, MigrationOutcome},
    settings::SettingsStore,
};

fn fixture(name: &str) -> LegacyStorageSnapshot {
    serde_json::from_str(match name {
        "valid" => include_str!("fixtures/legacy-settings/valid.json"),
        "partial" => include_str!("fixtures/legacy-settings/partial.json"),
        "corrupt" => include_str!("fixtures/legacy-settings/corrupt.json"),
        "future" => include_str!("fixtures/legacy-settings/future.json"),
        _ => panic!("unknown fixture"),
    })
    .unwrap()
}

fn temporary_directory(test: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("stg-legacy-{test}-{unique}"))
}

#[test]
fn valid_preview_maps_supported_values_without_exposing_secrets() {
    let source = fixture("valid");
    let original = serde_json::to_vec(&source).unwrap();
    let preview = LegacyMigrator::preview(&source, Default::default(), false);

    assert_eq!(preview.status, LegacyMigrationStatus::Ready);
    assert_eq!(
        preview.settings.theme,
        streamlink_twitch_gui_lib::domain::stream::Theme::Dark
    );
    assert_eq!(preview.settings.language, "de");
    assert_eq!(preview.settings.quality.maximum_height, Some(720));
    assert_eq!(preview.settings.player.path.as_deref(), Some("/bin/sh"));
    assert_eq!(
        preview.settings.player.arguments,
        ["--force-window=yes", "--keep-open"]
    );
    assert!(preview.settings.notifications.live_channels);
    assert_eq!(preview.channels.len(), 1);
    assert!(preview.changes.iter().any(|change| {
        change.field == "auth.access_token" && change.outcome == MigrationOutcome::SkippedSensitive
    }));
    assert!(preview.changes.iter().any(|change| {
        change.field == "streaming.twitch_api_header"
            && change.outcome == MigrationOutcome::SkippedSensitive
    }));
    let output = serde_json::to_string(&preview).unwrap();
    assert!(!output.contains("fixture-oauth-token"));
    assert!(!output.contains("fixture-sensitive-header"));
    assert_eq!(serde_json::to_vec(&source).unwrap(), original);
}

#[test]
fn partial_corrupt_and_future_data_are_safe_and_actionable() {
    let partial = LegacyMigrator::preview(&fixture("partial"), Default::default(), false);
    assert_eq!(
        partial.settings.theme,
        streamlink_twitch_gui_lib::domain::stream::Theme::Light
    );
    assert_eq!(
        partial.settings.quality.preference,
        streamlink_twitch_gui_lib::domain::stream::QualityPreference::AudioOnly
    );

    for name in ["corrupt", "future"] {
        let preview = LegacyMigrator::preview(&fixture(name), Default::default(), false);
        assert!(preview.changes.iter().any(|change| {
            matches!(
                change.outcome,
                MigrationOutcome::Invalid | MigrationOutcome::Unsupported
            )
        }));
    }
}

#[test]
fn quoted_player_arguments_are_preserved_and_malformed_quotes_are_not_imported() {
    let mut source = fixture("valid");
    source.settings = source.settings.map(|value| {
        value.replace(
            "--force-window=yes --keep-open",
            "--title 'Live channel name' --keep-open",
        )
    });
    let preview = LegacyMigrator::preview(&source, Default::default(), false);
    assert_eq!(
        preview.settings.player.arguments,
        ["--title", "Live channel name", "--keep-open"]
    );

    source.settings = source.settings.map(|value| {
        value.replace(
            "--title 'Live channel name' --keep-open",
            "--title 'unterminated",
        )
    });
    let preview = LegacyMigrator::preview(&source, Default::default(), false);
    assert!(preview.settings.player.arguments.is_empty());
    assert!(preview.changes.iter().any(|change| {
        change.field == "streaming.player.args" && change.outcome == MigrationOutcome::Unsupported
    }));
}

#[test]
fn import_requires_confirmation_is_one_time_and_never_mutates_legacy_data() {
    let directory = temporary_directory("confirm");
    fs::create_dir_all(&directory).unwrap();
    let legacy_path = directory.join("legacy.json");
    let source_bytes = include_bytes!("fixtures/legacy-settings/valid.json");
    fs::write(&legacy_path, source_bytes).unwrap();
    let source: LegacyStorageSnapshot = serde_json::from_slice(source_bytes).unwrap();
    let store = SettingsStore::new(directory.join("settings.json"));
    let marker = directory.join("legacy-migration-complete");
    let migrator = LegacyMigrator::new(store.clone(), marker.clone());

    assert!(migrator.import(&source, false).is_err());
    assert!(!marker.exists());
    let imported = migrator.import(&source, true).unwrap();
    assert_eq!(imported.status, LegacyMigrationStatus::Completed);
    assert_eq!(store.load().unwrap(), imported.settings);
    assert_eq!(fs::read(&legacy_path).unwrap(), source_bytes);

    let second = migrator.import(&fixture("partial"), true).unwrap();
    assert_eq!(second.status, LegacyMigrationStatus::AlreadyCompleted);
    assert_eq!(store.load().unwrap(), imported.settings);
    fs::remove_dir_all(directory).unwrap();
}
