use std::{ffi::OsString, fmt, time::Duration};

use serde_json::Value;

use crate::domain::stream::{StreamCapabilities, StreamCodec, StreamResolution, StreamVariant};

use super::{
    discovery::StreamlinkExecutable,
    process::{CaptureError, capture_command},
};

#[derive(Debug)]
pub enum InspectionError {
    InvalidUrl,
    MalformedJson(serde_json::Error),
    MissingStreams,
    Process(String),
    TimedOut,
    Unsuccessful(String),
}

impl fmt::Display for InspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl => formatter.write_str("stream URL must be a valid HTTP or HTTPS URL"),
            Self::MalformedJson(error) => write!(formatter, "invalid Streamlink JSON: {error}"),
            Self::MissingStreams => {
                formatter.write_str("Streamlink JSON does not contain a streams object")
            }
            Self::Process(message) => write!(formatter, "could not inspect streams: {message}"),
            Self::TimedOut => formatter.write_str("Streamlink inspection timed out"),
            Self::Unsuccessful(message) => {
                write!(formatter, "Streamlink inspection failed: {message}")
            }
        }
    }
}

pub fn inspect_streams(
    executable: &StreamlinkExecutable,
    url: &str,
    timeout: Duration,
) -> Result<StreamCapabilities, InspectionError> {
    let arguments = build_inspection_arguments(url)?;
    let output = capture_command(executable, &arguments, timeout).map_err(|error| match error {
        CaptureError::Process(error) => InspectionError::Process(error.to_string()),
        CaptureError::TimedOut => InspectionError::TimedOut,
    })?;
    if !output.success {
        return Err(InspectionError::Unsuccessful(
            output.stderr.trim().to_owned(),
        ));
    }
    parse_streams_json(&output.stdout)
}

impl std::error::Error for InspectionError {}

pub fn build_inspection_arguments(url: &str) -> Result<Vec<OsString>, InspectionError> {
    let remainder = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    if url.chars().any(char::is_control)
        || remainder.is_none_or(|value| {
            value
                .split('/')
                .next()
                .is_none_or(|host| host.is_empty() || host.chars().any(char::is_whitespace))
        })
    {
        return Err(InspectionError::InvalidUrl);
    }

    Ok(vec![
        "--no-config".into(),
        "--json".into(),
        "--twitch-supported-codecs".into(),
        "h264,h265,av1".into(),
        "--url".into(),
        url.into(),
    ])
}

pub fn parse_streams_json(json: &str) -> Result<StreamCapabilities, InspectionError> {
    let root: Value = serde_json::from_str(json).map_err(InspectionError::MalformedJson)?;
    let streams = root
        .get("streams")
        .and_then(Value::as_object)
        .ok_or(InspectionError::MissingStreams)?;
    let mut variants = Vec::new();

    for (name, stream) in streams {
        if is_alias(name) {
            continue;
        }
        let aliases = streams
            .iter()
            .filter(|(candidate, value)| is_alias(candidate) && *value == stream)
            .map(|(candidate, _)| candidate.clone())
            .collect();
        variants.push(parse_variant(name, stream, aliases));
    }

    Ok(StreamCapabilities {
        variants,
        supports_codec_selection: true,
    })
}

fn is_alias(name: &str) -> bool {
    matches!(
        name,
        "best" | "worst" | "best-unfiltered" | "worst-unfiltered"
    )
}

fn parse_variant(name: &str, stream: &Value, aliases: Vec<String>) -> StreamVariant {
    let label_dimensions = dimensions_from_label(name);
    let resolution = resolution_from_json(stream).or_else(|| {
        label_dimensions.map(|(height, _)| StreamResolution {
            width: height.saturating_mul(16).div_ceil(9),
            height,
        })
    });
    let fps = stream
        .get("fps")
        .or_else(|| stream.get("frame_rate"))
        .and_then(Value::as_f64)
        .or_else(|| label_dimensions.and_then(|(_, fps)| fps.map(f64::from)));
    let codec_text = stream
        .get("video_codec")
        .or_else(|| stream.get("codec"))
        .and_then(Value::as_str)
        .unwrap_or(name);
    let codec = if is_audio_label(name) {
        None
    } else {
        Some(parse_codec(codec_text))
    };
    let bitrate_kbps = stream
        .get("bitrate_kbps")
        .and_then(Value::as_u64)
        .or_else(|| {
            stream
                .get("bitrate")
                .and_then(Value::as_u64)
                .map(|value| value / 1000)
        })
        .and_then(|value| u32::try_from(value).ok());

    StreamVariant {
        name: name.to_owned(),
        resolution,
        fps,
        codec,
        bitrate_kbps,
        aliases,
    }
}

fn resolution_from_json(stream: &Value) -> Option<StreamResolution> {
    let resolution = stream.get("resolution")?;
    Some(StreamResolution {
        width: u32::try_from(resolution.get("width")?.as_u64()?).ok()?,
        height: u32::try_from(resolution.get("height")?.as_u64()?).ok()?,
    })
}

fn dimensions_from_label(label: &str) -> Option<(u32, Option<u32>)> {
    let bytes = label.as_bytes();
    for p_index in 1..bytes.len() {
        if bytes[p_index] != b'p' || !bytes[p_index - 1].is_ascii_digit() {
            continue;
        }
        let start = bytes[..p_index]
            .iter()
            .rposition(|byte| !byte.is_ascii_digit())
            .map_or(0, |index| index + 1);
        let height = label[start..p_index].parse().ok()?;
        let fps_end = bytes[p_index + 1..]
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .map_or(bytes.len(), |offset| p_index + 1 + offset);
        let fps = if fps_end > p_index + 1 {
            label[p_index + 1..fps_end].parse().ok()
        } else {
            None
        };
        return Some((height, fps));
    }
    None
}

fn parse_codec(value: &str) -> StreamCodec {
    let lowercase = value.to_ascii_lowercase();
    if lowercase.contains("av1") || lowercase.contains("av01") {
        StreamCodec::Av1
    } else if lowercase.contains("h265")
        || lowercase.contains("hevc")
        || lowercase.contains("hev1")
        || lowercase.contains("hvc1")
    {
        StreamCodec::H265
    } else if lowercase.contains("h264") || lowercase.contains("avc1") || lowercase.contains("avc")
    {
        StreamCodec::H264
    } else {
        StreamCodec::Unknown
    }
}

fn is_audio_label(label: &str) -> bool {
    let lowercase = label.to_ascii_lowercase();
    lowercase == "audio" || lowercase == "audio_only" || lowercase == "audio-only"
}
