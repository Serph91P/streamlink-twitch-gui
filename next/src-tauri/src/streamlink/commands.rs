use std::{path::PathBuf, sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    domain::stream::{QualityConstraints, QualityPreference, StreamCapabilities, StreamCodec},
    settings::SettingsState,
};

use super::{
    arguments::{PlaybackRequest, PlayerConfiguration, build_playback_arguments},
    discovery::detect_streamlink,
    inspect,
    process::{PlaybackProcess, launch_playback},
};

pub struct PlaybackState(pub Mutex<Option<PlaybackProcess>>);

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
    playback: tauri::State<'_, PlaybackState>,
    settings: tauri::State<'_, SettingsState>,
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
    let process =
        launch_playback(&detection.executable, arguments).map_err(|error| error.to_string())?;
    let mut active = playback
        .0
        .lock()
        .map_err(|_| "playback state is unavailable")?;
    if let Some(mut previous) = active.take() {
        previous.cancel().map_err(|error| error.to_string())?;
    }
    *active = Some(process);
    Ok(PlaybackResult {
        status: "running",
        diagnostics: Vec::new(),
    })
}

#[tauri::command]
pub fn stop_stream(playback: tauri::State<'_, PlaybackState>) -> Result<PlaybackResult, String> {
    let mut active = playback
        .0
        .lock()
        .map_err(|_| "playback state is unavailable")?;
    let diagnostics = if let Some(mut process) = active.take() {
        process.cancel().map_err(|error| error.to_string())?;
        process
            .diagnostics()
            .into_iter()
            .map(|line| line.message)
            .collect()
    } else {
        Vec::new()
    };
    Ok(PlaybackResult {
        status: "stopped",
        diagnostics,
    })
}
