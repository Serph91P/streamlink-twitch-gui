use std::ffi::OsString;

use streamlink_twitch_gui_lib::{
    domain::stream::StreamCodec,
    streamlink::inspect::{build_inspection_arguments, parse_streams_json},
};

#[test]
fn parses_streamlink_8_0_h264_contract() {
    let capabilities = parse_streams_json(include_str!(
        "fixtures/streams/streamlink-8.0-h264.synthetic.json"
    ))
    .unwrap();

    let variant = capabilities
        .variants
        .iter()
        .find(|variant| variant.name == "1080p60 (h264)")
        .unwrap();
    assert_eq!(variant.resolution.as_ref().unwrap().width, 1920);
    assert_eq!(variant.resolution.as_ref().unwrap().height, 1080);
    assert_eq!(variant.fps, Some(60.0));
    assert_eq!(variant.codec, Some(StreamCodec::H264));
    assert_eq!(variant.aliases, vec!["best"]);
}

#[test]
fn parses_modern_codecs_and_dynamic_1440p() {
    let capabilities = parse_streams_json(include_str!(
        "fixtures/streams/streamlink-8.4-modern-codecs.synthetic.json"
    ))
    .unwrap();

    for codec in [StreamCodec::H264, StreamCodec::H265, StreamCodec::Av1] {
        assert!(
            capabilities
                .variants
                .iter()
                .any(|variant| variant.codec == Some(codec.clone()))
        );
    }
    let h265 = capabilities
        .variants
        .iter()
        .find(|variant| variant.name == "1440p60 (hevc)")
        .unwrap();
    assert_eq!(h265.resolution.as_ref().unwrap().width, 2560);
    assert_eq!(h265.resolution.as_ref().unwrap().height, 1440);
    assert_eq!(h265.fps, Some(60.0));
    assert_eq!(h265.codec, Some(StreamCodec::H265));
    assert_eq!(h265.bitrate_kbps, Some(12000));
}

#[test]
fn preserves_unknown_future_labels() {
    let capabilities = parse_streams_json(include_str!(
        "fixtures/streams/streamlink-8.4-modern-codecs.synthetic.json"
    ))
    .unwrap();
    let future = capabilities
        .variants
        .iter()
        .find(|variant| variant.name == "future_ultra")
        .unwrap();

    assert_eq!(future.codec, Some(StreamCodec::Unknown));
    assert_eq!(future.resolution, None);
}

#[test]
fn inspection_uses_machine_output_and_all_supported_codecs() {
    let arguments = build_inspection_arguments("https://twitch.tv/example").unwrap();
    assert_eq!(
        arguments,
        [
            "--no-config",
            "--json",
            "--twitch-supported-codecs",
            "h264,h265,av1",
            "--url",
            "https://twitch.tv/example",
        ]
        .map(OsString::from)
    );
}

#[test]
fn rejects_malformed_machine_output() {
    assert!(parse_streams_json("not JSON").is_err());
    assert!(parse_streams_json(r#"{"plugin":"twitch"}"#).is_err());
}
