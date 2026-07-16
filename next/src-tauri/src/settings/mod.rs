use std::{
    fs::{self, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

pub use crate::domain::stream::Settings as AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum PlayerStatus {
    Unconfigured,
    ConfiguredUsable,
    ConfiguredUnavailable,
}

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
        validate(&settings, false)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        validate(settings, true)?;
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

    pub fn player_status(&self) -> Result<PlayerStatus, SettingsError> {
        if !self.path.exists() {
            return Ok(PlayerStatus::Unconfigured);
        }
        let value: serde_json::Value =
            serde_json::from_reader(BufReader::new(fs::File::open(&self.path)?))?;
        let settings: AppSettings = serde_json::from_value(value)?;
        validate(&settings, false)?;
        Ok(match settings.player.path {
            None => PlayerStatus::Unconfigured,
            Some(path) if executable_is_usable(Path::new(&path)) => PlayerStatus::ConfiguredUsable,
            Some(_) => PlayerStatus::ConfiguredUnavailable,
        })
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

fn validate(settings: &AppSettings, require_executable_files: bool) -> Result<(), SettingsError> {
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
            if require_executable_files && !executable_is_usable(Path::new(path)) {
                return Err(SettingsError::Validation(format!(
                    "{name} does not exist or is not an executable file"
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

fn executable_is_usable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        windows_executable_is_usable(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[cfg(windows)]
fn windows_executable_is_usable(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetBinaryTypeW;

    if !has_windows_executable_extension(path) {
        return false;
    }
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let mut binary_type = u32::MAX;
    let recognized = unsafe { GetBinaryTypeW(path.as_ptr(), &mut binary_type) != 0 };
    recognized && windows_binary_type_is_usable(binary_type)
}

#[cfg(any(test, windows))]
fn has_windows_executable_extension(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("com")
    })
}

#[cfg(any(test, windows))]
const fn windows_binary_type_is_usable(binary_type: u32) -> bool {
    matches!(binary_type, 0 | 6)
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
pub fn get_player_status(state: tauri::State<'_, SettingsState>) -> Result<PlayerStatus, String> {
    state.0.player_status().map_err(|error| error.to_string())
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

#[cfg(test)]
mod tests {
    use super::{has_windows_executable_extension, windows_binary_type_is_usable};
    use std::path::Path;

    #[test]
    fn windows_player_contract_accepts_only_native_executable_extensions() {
        assert!(has_windows_executable_extension(Path::new(
            "C:\\Tools\\mpv.EXE"
        )));
        assert!(has_windows_executable_extension(Path::new(
            "C:\\Tools\\player.com"
        )));
        assert!(!has_windows_executable_extension(Path::new(
            "C:\\Tools\\notes.txt"
        )));
        assert!(!has_windows_executable_extension(Path::new(
            "C:\\Tools\\player"
        )));
        assert!(windows_binary_type_is_usable(0));
        assert!(windows_binary_type_is_usable(6));
        assert!(!windows_binary_type_is_usable(1));
    }
}
