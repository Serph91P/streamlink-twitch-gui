pub mod domain;
pub mod streamlink;
pub mod twitch;

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
        .manage(twitch_state)
        .setup(|app| {
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
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Streamlink Twitch GUI");
}
