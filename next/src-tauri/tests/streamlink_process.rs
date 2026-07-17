use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use streamlink_twitch_gui_lib::streamlink::{
    arguments::BuiltArguments,
    discovery::StreamlinkExecutable,
    inspect::inspect_streams,
    process::{DiagnosticSource, ProcessStatus, launch_playback},
};

static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

fn fake_streamlink() -> PathBuf {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let output = std::env::temp_dir().join(format!(
        "streamlink-gui-fake-{}-{}{}",
        std::process::id(),
        FIXTURE_ID.fetch_add(1, Ordering::Relaxed),
        suffix
    ));
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_streamlink.rs");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let status = Command::new(rustc)
        .args([source.as_os_str(), "-o".as_ref(), output.as_os_str()])
        .status()
        .expect("rustc must compile the deterministic process fixture");
    assert!(status.success());
    output
}

fn executable() -> StreamlinkExecutable {
    StreamlinkExecutable {
        program: fake_streamlink().into_os_string(),
        prefix_arguments: Vec::new(),
    }
}

fn arguments(url: &str) -> BuiltArguments {
    BuiltArguments {
        execution: vec![
            "--url".into(),
            url.into(),
            "--twitch-api-header".into(),
            "Authorization=OAuth process-secret".into(),
        ],
        diagnostic: vec![
            "--url".into(),
            url.into(),
            "--twitch-api-header".into(),
            "Authorization=OAuth <redacted>".into(),
        ],
    }
}

#[test]
fn reports_status_and_redacts_piped_diagnostics() {
    let mut playback = launch_playback(&executable(), arguments("https://fixture.invalid/exit"))
        .expect("fake playback must launch");
    assert!(matches!(
        playback.status().unwrap(),
        ProcessStatus::Running { .. }
    ));

    let status = playback.wait_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        status,
        ProcessStatus::Exited {
            code: Some(0),
            success: true,
        }
    );
    let diagnostics = playback.diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|line| line.source == DiagnosticSource::Stdout)
    );
    assert!(
        diagnostics
            .iter()
            .any(|line| line.source == DiagnosticSource::Stderr)
    );
    assert!(
        diagnostics
            .iter()
            .all(|line| !line.message.contains("process-secret"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|line| line.message.contains("<redacted>"))
    );
}

#[test]
fn cancellation_returns_structured_status() {
    let mut playback = launch_playback(&executable(), arguments("https://fixture.invalid/wait"))
        .expect("fake playback must launch");

    assert_eq!(playback.cancel().unwrap(), ProcessStatus::Cancelled);
    assert_eq!(playback.status().unwrap(), ProcessStatus::Cancelled);
}

#[test]
fn bounded_wait_does_not_treat_logs_as_success_protocol() {
    let mut playback = launch_playback(&executable(), arguments("https://fixture.invalid/wait"))
        .expect("fake playback must launch");

    let pid = playback.id();
    assert_eq!(
        playback.wait_timeout(Duration::from_millis(50)).unwrap(),
        ProcessStatus::Running { pid }
    );
    playback.cancel().unwrap();
}

#[test]
fn rejects_early_exit_with_sanitized_actionable_diagnostics() {
    let playback = launch_playback(
        &executable(),
        arguments("https://fixture.invalid/fail?token=request-secret"),
    )
    .expect("fake playback must launch");

    let error = playback.accept(Duration::from_secs(5)).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("exited before playback started"));
    assert!(message.contains("exit code 7"));
    assert!(!message.contains("request-secret"));
    assert!(!message.contains("process-secret"));
    assert!(!message.contains("raw-output-secret"));
    assert!(!message.contains("raw-query-secret"));
    assert!(!message.contains("standalone-secret"));
    assert!(!message.contains("cdn.example.test"));
}

#[test]
fn executes_bounded_machine_readable_inspection() {
    let capabilities = inspect_streams(
        &executable(),
        "https://fixture.invalid/inspect",
        Duration::from_secs(5),
    )
    .unwrap();

    assert_eq!(capabilities.variants[0].name, "1440p60 (av1)");
    assert_eq!(capabilities.variants[0].aliases, vec!["best"]);
}
