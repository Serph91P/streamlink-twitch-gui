use std::{ffi::OsString, fmt};

use crate::domain::stream::{QualityConstraints, QualityPreference, StreamCodec};

#[derive(Clone, PartialEq, Eq)]
pub struct PlaybackOAuth(String);

impl PlaybackOAuth {
    pub fn new(token: String) -> Result<Self, ArgumentError> {
        validate_text("playback OAuth token", &token)?;
        if token.trim().is_empty() {
            return Err(ArgumentError::InvalidValue(
                "playback OAuth token cannot be empty".into(),
            ));
        }
        Ok(Self(token))
    }
}

impl fmt::Debug for PlaybackOAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PlaybackOAuth(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerConfiguration {
    pub path: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackRequest {
    pub url: String,
    pub variant_name: Option<String>,
    pub quality: QualityConstraints,
    pub player: Option<PlayerConfiguration>,
    pub codecs: Vec<StreamCodec>,
    pub playback_oauth: Option<PlaybackOAuth>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BuiltArguments {
    pub execution: Vec<OsString>,
    pub diagnostic: Vec<String>,
}

impl fmt::Debug for BuiltArguments {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BuiltArguments")
            .field("arguments", &self.diagnostic)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentError {
    InvalidValue(String),
}

impl fmt::Display for ArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ArgumentError {}

pub fn build_playback_arguments(
    request: &PlaybackRequest,
) -> Result<BuiltArguments, ArgumentError> {
    validate_url(&request.url)?;
    let mut execution = vec![
        OsString::from("--no-config"),
        OsString::from("--url"),
        OsString::from(&request.url),
        OsString::from("--default-stream"),
        OsString::from(if let Some(variant_name) = &request.variant_name {
            validate_text("stream variant", variant_name)?;
            if variant_name.trim().is_empty() {
                return Err(ArgumentError::InvalidValue(
                    "stream variant cannot be empty".into(),
                ));
            }
            variant_name.as_str()
        } else {
            match request.quality.preference {
                QualityPreference::Best => "best",
                QualityPreference::Worst => "worst",
                QualityPreference::AudioOnly => "audio_only",
            }
        }),
    ];

    if request.quality.maximum_height == Some(0) || request.quality.maximum_fps == Some(0) {
        return Err(ArgumentError::InvalidValue(
            "quality limits must be greater than zero".into(),
        ));
    }
    if request.quality.maximum_height.is_some() || request.quality.maximum_fps.is_some() {
        let limit = match (request.quality.maximum_height, request.quality.maximum_fps) {
            (Some(height), Some(fps)) => format!(">{height}p{fps}"),
            (Some(height), None) => format!(">{height}p"),
            (None, Some(fps)) => format!(">{fps}fps"),
            (None, None) => unreachable!(),
        };
        execution.extend(["--stream-sorting-excludes".into(), limit.into()]);
    }

    if let Some(player) = &request.player {
        validate_text("player path", &player.path)?;
        if player.path.trim().is_empty() {
            return Err(ArgumentError::InvalidValue(
                "player path cannot be empty".into(),
            ));
        }
        execution.extend(["--player".into(), player.path.clone().into()]);
        if !player.arguments.is_empty() {
            for argument in &player.arguments {
                validate_text("player argument", argument)?;
            }
            let arguments =
                shlex::try_join(player.arguments.iter().map(String::as_str)).map_err(|_| {
                    ArgumentError::InvalidValue("player arguments cannot contain NUL bytes".into())
                })?;
            execution.extend(["--player-args".into(), arguments.into()]);
        }
    }

    let mut codecs = Vec::new();
    for codec in &request.codecs {
        let name = match codec {
            StreamCodec::H264 => Some("h264"),
            StreamCodec::H265 => Some("h265"),
            StreamCodec::Av1 => Some("av1"),
            StreamCodec::Unknown => None,
        };
        if let Some(name) = name
            && !codecs.contains(&name)
        {
            codecs.push(name);
        }
    }
    if !codecs.is_empty() {
        execution.extend(["--twitch-supported-codecs".into(), codecs.join(",").into()]);
    }

    let mut diagnostic = execution
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if let Some(oauth) = &request.playback_oauth {
        execution.extend([
            "--twitch-api-header".into(),
            format!("Authorization=OAuth {}", oauth.0).into(),
        ]);
        diagnostic.extend([
            "--twitch-api-header".into(),
            "Authorization=OAuth <redacted>".into(),
        ]);
    }

    Ok(BuiltArguments {
        execution,
        diagnostic,
    })
}

fn validate_url(url: &str) -> Result<(), ArgumentError> {
    validate_text("stream URL", url)?;
    let authority = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|remainder| remainder.split('/').next())
        .filter(|authority| !authority.is_empty() && !authority.chars().any(char::is_whitespace));
    if authority.is_none() {
        return Err(ArgumentError::InvalidValue(
            "stream URL must use HTTP or HTTPS and include a host".into(),
        ));
    }
    Ok(())
}

fn validate_text(name: &str, value: &str) -> Result<(), ArgumentError> {
    if value.chars().any(char::is_control) {
        return Err(ArgumentError::InvalidValue(format!(
            "{name} cannot contain control characters"
        )));
    }
    Ok(())
}
