mod legacy;

pub use legacy::{
    LegacyChannelPreview, LegacyMigrationError, LegacyMigrationPreview, LegacyMigrationStatus,
    LegacyMigrator, LegacyStorageSnapshot, MigrationChange, MigrationOutcome,
};

#[cfg(feature = "desktop")]
pub struct LegacyMigrationState(pub LegacyMigrator);

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn preview_legacy_migration(
    state: tauri::State<'_, LegacyMigrationState>,
    snapshot: LegacyStorageSnapshot,
) -> Result<LegacyMigrationPreview, String> {
    state
        .0
        .preview_current(&snapshot)
        .map_err(|error| error.to_string())
}

#[cfg(feature = "desktop")]
#[tauri::command]
pub fn confirm_legacy_migration(
    app: tauri::AppHandle,
    state: tauri::State<'_, LegacyMigrationState>,
    runtime: tauri::State<'_, crate::desktop::RuntimeSettings>,
    snapshot: LegacyStorageSnapshot,
    confirmed: bool,
) -> Result<LegacyMigrationPreview, String> {
    let preview = state
        .0
        .import(&snapshot, confirmed)
        .map_err(|error| error.to_string())?;
    crate::desktop::apply_runtime_settings(&app, &runtime, &preview.settings)?;
    Ok(preview)
}
