use std::ffi::{OsStr, OsString};

use streamlink_twitch_gui_lib::{
    domain::stream::{QualityConstraints, QualityPreference, StreamCodec},
    streamlink::arguments::{
        PlaybackOAuth, PlaybackRequest, PlayerConfiguration, build_playback_arguments,
    },
};

fn strings(arguments: &[OsString]) -> Vec<&OsStr> {
    arguments.iter().map(OsString::as_os_str).collect()
}

#[test]
fn builds_direct_arguments_for_dynamic_playback_settings() {
    let request = PlaybackRequest {
        url: "https://www.twitch.tv/example".into(),
        variant_name: None,
        quality: QualityConstraints {
            preference: QualityPreference::Best,
            maximum_height: Some(1440),
            maximum_fps: Some(60),
        },
        player: Some(PlayerConfiguration {
            path: "/opt/Video Player/player".into(),
            arguments: vec!["--fullscreen".into(), "--title=Example Stream".into()],
        }),
        codecs: vec![
            StreamCodec::H264,
            StreamCodec::H265,
            StreamCodec::Av1,
            StreamCodec::Unknown,
        ],
        playback_oauth: Some(PlaybackOAuth::new("playback-secret".into()).unwrap()),
    };

    let built = build_playback_arguments(&request).unwrap();

    assert_eq!(
        strings(&built.execution),
        vec![
            OsStr::new("--no-config"),
            OsStr::new("--url"),
            OsStr::new("https://www.twitch.tv/example"),
            OsStr::new("--default-stream"),
            OsStr::new("best"),
            OsStr::new("--stream-sorting-excludes"),
            OsStr::new(">1440p60"),
            OsStr::new("--player"),
            OsStr::new("/opt/Video Player/player"),
            OsStr::new("--player-args"),
            OsStr::new("--fullscreen --title=Example Stream"),
            OsStr::new("--twitch-supported-codecs"),
            OsStr::new("h264,h265,av1"),
            OsStr::new("--twitch-api-header"),
            OsStr::new("Authorization=OAuth playback-secret"),
        ]
    );
    assert!(!built.diagnostic.join(" ").contains("playback-secret"));
    assert!(
        built
            .diagnostic
            .join(" ")
            .contains("Authorization=OAuth <redacted>")
    );
}

#[test]
fn maps_quality_preferences_without_static_quality_names() {
    for (preference, expected) in [
        (QualityPreference::Best, "best"),
        (QualityPreference::Worst, "worst"),
        (QualityPreference::AudioOnly, "audio_only"),
    ] {
        let request = PlaybackRequest {
            url: "http://localhost/channel".into(),
            variant_name: None,
            quality: QualityConstraints {
                preference,
                maximum_height: None,
                maximum_fps: None,
            },
            player: None,
            codecs: vec![],
            playback_oauth: None,
        };

        let built = build_playback_arguments(&request).unwrap();
        let quality_index = built
            .execution
            .iter()
            .position(|argument| argument == "--default-stream")
            .unwrap();
        assert_eq!(built.execution[quality_index + 1], expected);
        assert!(
            !built
                .execution
                .iter()
                .any(|argument| argument == "--twitch-supported-codecs")
        );
    }
}

#[test]
fn rejects_unsafe_urls_and_control_characters() {
    let base = PlaybackRequest {
        url: "file:///tmp/video".into(),
        variant_name: None,
        quality: QualityConstraints {
            preference: QualityPreference::Best,
            maximum_height: None,
            maximum_fps: None,
        },
        player: None,
        codecs: vec![StreamCodec::H264],
        playback_oauth: None,
    };
    assert!(build_playback_arguments(&base).is_err());

    let mut controlled = base;
    controlled.url = "https://twitch.tv/example\n--player=bad".into();
    assert!(build_playback_arguments(&controlled).is_err());
    assert!(PlaybackOAuth::new("token\rvalue".into()).is_err());
}
