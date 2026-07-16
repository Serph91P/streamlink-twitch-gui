use streamlink_twitch_gui_lib::startup::StartupFailure;

#[test]
fn startup_failure_is_actionable_and_safe_for_native_dialogs() {
    let failure = StartupFailure::new("Twitch client ID is not\0configured");

    assert_eq!(failure.title(), "Streamlink Twitch GUI");
    assert_eq!(
        failure.message(),
        "The application could not start.\n\nTwitch client ID is not configured\n\nCheck the installation and settings, then try again."
    );
    assert!(!failure.message().contains('\0'));
}
