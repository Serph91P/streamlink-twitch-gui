use std::{
    fs::{self, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

pub use crate::domain::stream::Settings as AppSettings;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported settings schema version {0}")]
    Schema(u32),
    #[error("{0}")]
    Validation(String),
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppSettings, SettingsError> {
        if !self.path.exists() {
            return Ok(AppSettings::default());
        }
        let value: serde_json::Value =
            serde_json::from_reader(BufReader::new(fs::File::open(&self.path)?))?;
        let schema_version = value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if schema_version != 1 {
            return Err(SettingsError::Schema(
                u32::try_from(schema_version).unwrap_or(u32::MAX),
            ));
        }
        let settings: AppSettings = serde_json::from_value(value)?;
        validate(&settings)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        validate(settings)?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let temporary = self.path.with_extension("json.tmp");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, settings)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        replace_file(&temporary, &self.path)?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn validate(settings: &AppSettings) -> Result<(), SettingsError> {
    if settings.schema_version != 1 {
        return Err(SettingsError::Schema(settings.schema_version));
    }
    for (name, path) in [
        ("Streamlink executable", settings.streamlink_path.as_deref()),
        ("player executable", settings.player.path.as_deref()),
    ] {
        if let Some(path) = path {
            validate_text(name, path)?;
            if path.trim().is_empty() {
                return Err(SettingsError::Validation(format!("{name} cannot be empty")));
            }
            if !Path::new(path).is_file() {
                return Err(SettingsError::Validation(format!(
                    "{name} does not exist or is not a file"
                )));
            }
        }
    }
    for argument in &settings.player.arguments {
        validate_text("player argument", argument)?;
    }
    validate_text("language", &settings.language)?;
    validate_text("global hotkey", &settings.hotkey.accelerator)?;
    if settings.language.trim().is_empty() {
        return Err(SettingsError::Validation("language cannot be empty".into()));
    }
    if settings.hotkey.enabled && settings.hotkey.accelerator.trim().is_empty() {
        return Err(SettingsError::Validation(
            "enabled global hotkey cannot be empty".into(),
        ));
    }
    if settings.quality.maximum_height == Some(0) || settings.quality.maximum_fps == Some(0) {
        return Err(SettingsError::Validation(
            "quality limits must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn validate_text(name: &str, value: &str) -> Result<(), SettingsError> {
    if value.chars().any(char::is_control) {
        return Err(SettingsError::Validation(format!(
            "{name} cannot contain control characters"
        )));
    }
    Ok(())
}

#[cfg(feature = "desktop")]
pub struct SettingsState(pub SettingsStore);

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, SettingsState>) -> Result<AppSettings, String> {
    state.0.load().map_err(|error| error.to_string())
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, SettingsState>,
    runtime: tauri::State<'_, crate::desktop::RuntimeSettings>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    state.0.save(&settings).map_err(|error| error.to_string())?;
    crate::desktop::apply_runtime_settings(&app, &runtime, &settings)?;
    Ok(settings)
}
