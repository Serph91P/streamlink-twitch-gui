#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ToggleWindow,
    PlayOrStop,
    Quit,
}

pub fn tray_action(menu_id: &str) -> Option<TrayAction> {
    match menu_id {
        "toggle-window" => Some(TrayAction::ToggleWindow),
        "play-or-stop" => Some(TrayAction::PlayOrStop),
        "quit" => Some(TrayAction::Quit),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackAction {
    Stop,
    Replay,
    ShowWindow,
}

pub fn playback_action(is_active: bool, has_previous: bool) -> PlaybackAction {
    if is_active {
        PlaybackAction::Stop
    } else if has_previous {
        PlaybackAction::Replay
    } else {
        PlaybackAction::ShowWindow
    }
}

pub fn should_notify_playback_error(
    settings: &crate::domain::stream::NotificationSettings,
) -> bool {
    settings.playback_errors
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyUpdate<'a> {
    Keep,
    Disable,
    Replace(&'a str),
}

pub fn hotkey_update<'a>(
    current: Option<&str>,
    settings: &'a crate::domain::stream::HotkeySettings,
) -> HotkeyUpdate<'a> {
    if settings.enabled {
        if current == Some(settings.accelerator.as_str()) {
            HotkeyUpdate::Keep
        } else {
            HotkeyUpdate::Replace(&settings.accelerator)
        }
    } else if current.is_some() {
        HotkeyUpdate::Disable
    } else {
        HotkeyUpdate::Keep
    }
}

#[cfg(feature = "desktop")]
pub struct RuntimeSettings {
    playback_error_notifications: std::sync::atomic::AtomicBool,
    registered_hotkey: std::sync::Mutex<Option<String>>,
}

#[cfg(feature = "desktop")]
impl RuntimeSettings {
    pub fn new(settings: &crate::settings::AppSettings) -> Self {
        Self {
            playback_error_notifications: std::sync::atomic::AtomicBool::new(
                should_notify_playback_error(&settings.notifications),
            ),
            registered_hotkey: std::sync::Mutex::new(None),
        }
    }

    pub fn apply_notifications(&self, settings: &crate::settings::AppSettings) {
        self.playback_error_notifications.store(
            should_notify_playback_error(&settings.notifications),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn playback_error_notifications(&self) -> bool {
        self.playback_error_notifications
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "desktop")]
pub fn apply_runtime_settings(
    app: &tauri::AppHandle,
    runtime: &RuntimeSettings,
    settings: &crate::settings::AppSettings,
) -> Result<(), String> {
    runtime.apply_notifications(settings);
    apply_hotkey(app, runtime, &settings.hotkey)
}

#[cfg(feature = "desktop")]
fn apply_hotkey(
    app: &tauri::AppHandle,
    runtime: &RuntimeSettings,
    settings: &crate::domain::stream::HotkeySettings,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let current = runtime
        .registered_hotkey
        .lock()
        .map_err(|_| "hotkey runtime state is unavailable")?
        .clone();
    let update = hotkey_update(current.as_deref(), settings);
    if matches!(update, HotkeyUpdate::Keep) {
        return Ok(());
    }
    if let Some(accelerator) = current {
        app.global_shortcut()
            .unregister(accelerator.as_str())
            .map_err(|error| error.to_string())?;
        *runtime
            .registered_hotkey
            .lock()
            .map_err(|_| "hotkey runtime state is unavailable")? = None;
    }
    if let HotkeyUpdate::Replace(accelerator) = update {
        app.global_shortcut()
            .on_shortcut(accelerator, |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    trigger_play_or_stop(app);
                }
            })
            .map_err(|error| error.to_string())?;
        *runtime
            .registered_hotkey
            .lock()
            .map_err(|_| "hotkey runtime state is unavailable")? = Some(accelerator.to_owned());
    }
    Ok(())
}

#[cfg(feature = "desktop")]
pub fn notify_playback_error(app: &tauri::AppHandle, runtime: &RuntimeSettings, message: &str) {
    if !runtime.playback_error_notifications() {
        return;
    }
    use tauri_plugin_notification::NotificationExt;

    if let Err(error) = app
        .notification()
        .builder()
        .title("Playback error")
        .body(message)
        .show()
    {
        eprintln!("playback error notification failed: {error}");
    }
}

#[cfg(feature = "desktop")]
fn toggle_window(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_owned())?;
    if window.is_visible().map_err(|error| error.to_string())? {
        window.hide().map_err(|error| error.to_string())
    } else {
        show_window(app)
    }
}

#[cfg(feature = "desktop")]
fn show_window(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_owned())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[cfg(feature = "desktop")]
pub fn trigger_play_or_stop(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;

        let result = crate::streamlink::commands::play_or_stop(
            app.state::<crate::streamlink::commands::PlaybackState>()
                .inner(),
        );
        match result {
            Ok(PlaybackAction::ShowWindow) => {
                if let Err(error) = show_window(&app) {
                    eprintln!("play-or-stop failed: {error}");
                }
            }
            Ok(_) => {}
            Err(error) => eprintln!("play-or-stop failed: {error}"),
        }
    });
}

#[cfg(feature = "desktop")]
pub fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{menu::MenuItem, tray::TrayIconBuilder};

    let toggle = MenuItem::with_id(app, "toggle-window", "Show or hide", true, None::<&str>)?;
    let playback = MenuItem::with_id(app, "play-or-stop", "Play or stop", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = tauri::menu::Menu::with_items(app, &[&toggle, &playback, &quit])?;
    TrayIconBuilder::new()
        .tooltip("Streamlink Twitch GUI")
        .menu(&menu)
        .on_menu_event(|app, event| match tray_action(event.id().as_ref()) {
            Some(TrayAction::ToggleWindow) => {
                if let Err(error) = toggle_window(app) {
                    eprintln!("tray action failed: {error}");
                }
            }
            Some(TrayAction::PlayOrStop) => trigger_play_or_stop(app),
            Some(TrayAction::Quit) => app.exit(0),
            None => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        HotkeyUpdate, PlaybackAction, TrayAction, hotkey_update, playback_action,
        should_notify_playback_error, tray_action,
    };

    #[test]
    fn maps_only_supported_tray_menu_actions() {
        assert_eq!(tray_action("toggle-window"), Some(TrayAction::ToggleWindow));
        assert_eq!(tray_action("play-or-stop"), Some(TrayAction::PlayOrStop));
        assert_eq!(tray_action("quit"), Some(TrayAction::Quit));
        assert_eq!(tray_action("unexpected"), None);
    }

    #[test]
    fn play_or_stop_uses_active_or_previous_playback_state() {
        assert_eq!(playback_action(true, true), PlaybackAction::Stop);
        assert_eq!(playback_action(false, true), PlaybackAction::Replay);
        assert_eq!(playback_action(false, false), PlaybackAction::ShowWindow);
    }

    #[test]
    fn playback_error_notifications_follow_runtime_settings() {
        let mut settings = crate::settings::AppSettings::default();
        assert!(should_notify_playback_error(&settings.notifications));
        settings.notifications.playback_errors = false;
        assert!(!should_notify_playback_error(&settings.notifications));
    }

    #[test]
    fn hotkey_updates_replace_disable_or_keep_the_registration() {
        let mut settings = crate::settings::AppSettings::default();
        assert_eq!(hotkey_update(None, &settings.hotkey), HotkeyUpdate::Keep);

        settings.hotkey.enabled = true;
        assert_eq!(
            hotkey_update(None, &settings.hotkey),
            HotkeyUpdate::Replace("Ctrl+Shift+S")
        );
        assert_eq!(
            hotkey_update(Some("Ctrl+Shift+S"), &settings.hotkey),
            HotkeyUpdate::Keep
        );

        settings.hotkey.enabled = false;
        assert_eq!(
            hotkey_update(Some("Ctrl+Shift+S"), &settings.hotkey),
            HotkeyUpdate::Disable
        );
    }
}
