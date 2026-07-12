use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use streamlink_twitch_gui_lib::settings::{AppSettings, SettingsStore};

fn temp_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("streamlink-gui-settings-{nonce}"))
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn round_trips_schema_versioned_settings_with_atomic_replacement() {
    let directory = temp_directory();
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    let store = SettingsStore::new(path.clone());
    let mut settings = AppSettings::default();
    settings.quality.maximum_height = Some(1440);
    settings.codec_preference.preferred =
        Some(streamlink_twitch_gui_lib::domain::stream::StreamCodec::Av1);

    store.save(&settings).unwrap();

    assert_eq!(store.load().unwrap(), settings);
    assert!(!directory.join("settings.json.tmp").exists());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(path).unwrap()).unwrap()["schemaVersion"],
        1
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn rejects_invalid_executables_and_argument_control_characters() {
    let directory = temp_directory();
    fs::create_dir_all(&directory).unwrap();
    let store = SettingsStore::new(directory.join("settings.json"));
    let mut missing = AppSettings::default();
    missing.streamlink_path = Some(
        directory
            .join("missing-streamlink")
            .to_string_lossy()
            .into_owned(),
    );
    assert!(
        store
            .save(&missing)
            .unwrap_err()
            .to_string()
            .contains("does not exist")
    );

    let mut unsafe_argument = AppSettings::default();
    unsafe_argument.player.arguments = vec!["--title=line\nnext".into()];
    assert!(
        store
            .save(&unsafe_argument)
            .unwrap_err()
            .to_string()
            .contains("control characters")
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn reports_unsupported_schema_without_overwriting_the_file() {
    let directory = temp_directory();
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    fs::write(&path, r#"{"schemaVersion":99}"#).unwrap();
    let store = SettingsStore::new(path.clone());

    assert!(
        store
            .load()
            .unwrap_err()
            .to_string()
            .contains("schema version")
    );
    assert_eq!(fs::read_to_string(path).unwrap(), r#"{"schemaVersion":99}"#);
    fs::remove_dir_all(directory).unwrap();
}
