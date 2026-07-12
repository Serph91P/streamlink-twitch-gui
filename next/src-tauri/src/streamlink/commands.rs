use std::{path::PathBuf, sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    desktop::{PlaybackAction, playback_action},
    domain::stream::{QualityConstraints, QualityPreference, StreamCapabilities, StreamCodec},
    settings::SettingsState,
};

use super::{
    arguments::{BuiltArguments, PlaybackRequest, PlayerConfiguration, build_playback_arguments},
    discovery::{StreamlinkExecutable, detect_streamlink},
    inspect,
    process::{PlaybackProcess, launch_playback},
};

#[derive(Clone)]
struct PreviousPlayback {
    executable: StreamlinkExecutable,
    arguments: BuiltArguments,
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn previous_playback_preserves_typed_retry_state() {
        let previous = PreviousPlayback {
            executable: StreamlinkExecutable {
                program: OsString::from("python"),
                prefix_arguments: vec![OsString::from("-m"), OsString::from("streamlink")],
            },
            arguments: BuiltArguments {
                execution: vec![
                    OsString::from("--url"),
                    OsString::from("https://example.test"),
                ],
                diagnostic: vec!["--url".into(), "https://example.test".into()],
            },
        };

        let retry = previous.clone();
        assert_eq!(retry.executable, previous.executable);
        assert_eq!(retry.arguments, previous.arguments);
    }
}

#[derive(Default)]
struct PlaybackRuntime {
    active: Option<PlaybackProcess>,
    previous: Option<PreviousPlayback>,
}

#[derive(Default)]
pub struct PlaybackState(Mutex<PlaybackRuntime>);

impl PlaybackState {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchRequest {
    url: String,
    variant_name: String,
    codecs: Vec<StreamCodec>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackResult {
    status: &'static str,
    diagnostics: Vec<String>,
}

#[tauri::command]
pub async fn inspect_streams(
    settings: tauri::State<'_, SettingsState>,
    url: String,
) -> Result<StreamCapabilities, String> {
    let selected = settings
        .0
        .load()
        .map_err(|error| error.to_string())?
        .streamlink_path
        .map(PathBuf::from);
    tauri::async_runtime::spawn_blocking(move || {
        let executable = detect_streamlink(selected, Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        inspect::inspect_streams(&executable.executable, &url, Duration::from_secs(15))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn launch_stream(
    app: tauri::AppHandle,
    playback: tauri::State<'_, PlaybackState>,
    settings: tauri::State<'_, SettingsState>,
    runtime: tauri::State<'_, crate::desktop::RuntimeSettings>,
    request: LaunchRequest,
) -> Result<PlaybackResult, String> {
    let result = launch_stream_inner(&playback, &settings, request);
    if let Err(error) = &result {
        crate::desktop::notify_playback_error(&app, &runtime, error);
    }
    result
}

fn launch_stream_inner(
    playback: &PlaybackState,
    settings: &SettingsState,
    request: LaunchRequest,
) -> Result<PlaybackResult, String> {
    let settings = settings.0.load().map_err(|error| error.to_string())?;
    let detection = detect_streamlink(
        settings.streamlink_path.map(PathBuf::from),
        Duration::from_secs(3),
    )
    .map_err(|error| error.to_string())?;
    let player = settings.player.path.map(|path| PlayerConfiguration {
        path,
        arguments: settings.player.arguments,
    });
    let arguments = build_playback_arguments(&PlaybackRequest {
        url: request.url,
        variant_name: Some(request.variant_name),
        quality: QualityConstraints {
            preference: QualityPreference::Best,
            maximum_height: settings.quality.maximum_height,
            maximum_fps: settings.quality.maximum_fps,
        },
        player,
        codecs: request.codecs,
        playback_oauth: None,
    })
    .map_err(|error| error.to_string())?;
    let process = launch_playback(&detection.executable, arguments.clone())
        .map_err(|error| error.to_string())?;
    let mut active = playback
        .0
        .lock()
        .map_err(|_| "playback state is unavailable")?;
    if let Some(mut previous) = active.active.take() {
        previous.cancel().map_err(|error| error.to_string())?;
    }
    active.active = Some(process);
    active.previous = Some(PreviousPlayback {
        executable: detection.executable,
        arguments,
    });
    Ok(PlaybackResult {
        status: "running",
        diagnostics: Vec::new(),
    })
}

#[tauri::command]
pub fn stop_stream(playback: tauri::State<'_, PlaybackState>) -> Result<PlaybackResult, String> {
    let diagnostics = stop_playback(&playback)?;
    Ok(PlaybackResult {
        status: "stopped",
        diagnostics,
    })
}

fn stop_playback(playback: &PlaybackState) -> Result<Vec<String>, String> {
    let mut runtime = playback
        .0
        .lock()
        .map_err(|_| "playback state is unavailable")?;
    let diagnostics = if let Some(mut process) = runtime.active.take() {
        process.cancel().map_err(|error| error.to_string())?;
        process
            .diagnostics()
            .into_iter()
            .map(|line| line.message)
            .collect()
    } else {
        Vec::new()
    };
    Ok(diagnostics)
}

pub fn play_or_stop(playback: &PlaybackState) -> Result<PlaybackAction, String> {
    let mut runtime = playback
        .0
        .lock()
        .map_err(|_| "playback state is unavailable")?;
    let action = playback_action(runtime.active.is_some(), runtime.previous.is_some());
    match action {
        PlaybackAction::Stop => {
            if let Some(mut process) = runtime.active.take() {
                process.cancel().map_err(|error| error.to_string())?;
            }
        }
        PlaybackAction::Replay => {
            let previous = runtime
                .previous
                .clone()
                .ok_or_else(|| "previous playback is unavailable".to_owned())?;
            runtime.active = Some(
                launch_playback(&previous.executable, previous.arguments)
                    .map_err(|error| error.to_string())?,
            );
        }
        PlaybackAction::ShowWindow => {}
    }
    Ok(action)
}
