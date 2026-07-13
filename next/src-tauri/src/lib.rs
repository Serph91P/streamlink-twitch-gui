pub mod desktop;
pub mod domain;
pub mod migration;
pub mod settings;
pub mod streamlink;
pub mod twitch;
pub mod updater_signature;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    let twitch_state = match twitch::commands::TwitchState::new(
        option_env!("TWITCH_CLIENT_ID").unwrap_or_default(),
    ) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(twitch_state)
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings_store = settings::SettingsStore::new(settings_path);
            let startup_settings = settings_store.load()?;
            let migration_marker = app
                .path()
                .app_config_dir()?
                .join("legacy-migration-complete");
            app.manage(migration::LegacyMigrationState(
                migration::LegacyMigrator::new(settings_store.clone(), migration_marker),
            ));
            app.manage(settings::SettingsState(settings_store));
            app.manage(desktop::RuntimeSettings::new(&startup_settings));
            app.manage(streamlink::commands::PlaybackState::new());
            desktop::apply_runtime_settings(
                app.handle(),
                app.state::<desktop::RuntimeSettings>().inner(),
                &startup_settings,
            )
            .map_err(std::io::Error::other)?;
            desktop::build_tray(app.handle())?;
            let runner = app
                .state::<twitch::commands::TwitchState>()
                .validation_runner();
            tauri::async_runtime::spawn(twitch::commands::run_validation_schedule(runner));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            twitch::commands::get_twitch_session,
            twitch::commands::begin_twitch_login,
            twitch::commands::poll_twitch_login,
            twitch::commands::sign_out_twitch,
            twitch::commands::twitch_users,
            twitch::commands::twitch_streams,
            twitch::commands::twitch_followed_streams,
            twitch::commands::twitch_followed_channels,
            twitch::commands::twitch_top_games,
            twitch::commands::twitch_search_channels,
            twitch::commands::twitch_search_categories,
            streamlink::commands::inspect_streams,
            streamlink::commands::launch_stream,
            streamlink::commands::stop_stream,
            settings::get_settings,
            settings::save_settings,
            migration::preview_legacy_migration,
            migration::confirm_legacy_migration,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Streamlink Twitch GUI");
}
