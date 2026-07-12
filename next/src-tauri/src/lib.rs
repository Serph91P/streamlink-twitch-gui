pub mod domain;
pub mod settings;
pub mod streamlink;
pub mod twitch;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::Mutex;

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
        .manage(twitch_state)
        .setup(|app| {
            tauri::tray::TrayIconBuilder::new()
                .tooltip("Streamlink Twitch GUI")
                .build(app)?;
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            app.manage(settings::SettingsState(settings::SettingsStore::new(
                settings_path,
            )));
            app.manage(streamlink::commands::PlaybackState(Mutex::new(None)));
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
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Streamlink Twitch GUI");
}
