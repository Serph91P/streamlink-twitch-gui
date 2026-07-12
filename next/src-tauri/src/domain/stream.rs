use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamCodec {
    H264,
    H265,
    Av1,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamVariant {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<StreamResolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<StreamCodec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCapabilities {
    pub variants: Vec<StreamVariant>,
    pub supports_codec_selection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreference {
    Best,
    Worst,
    #[serde(rename = "audioOnly")]
    AudioOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QualityConstraints {
    pub preference: QualityPreference,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_fps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodecPreference {
    pub allowed: Vec<StreamCodec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred: Option<StreamCodec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationSettings {
    pub live_channels: bool,
    pub playback_errors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HotkeySettings {
    pub enabled: bool,
    pub accelerator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streamlink_path: Option<String>,
    pub quality: QualityConstraints,
    pub codec_preference: CodecPreference,
    pub player: PlayerSettings,
    pub theme: Theme,
    pub language: String,
    pub notifications: NotificationSettings,
    pub hotkey: HotkeySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            streamlink_path: None,
            quality: QualityConstraints {
                preference: QualityPreference::Best,
                maximum_height: None,
                maximum_fps: None,
            },
            codec_preference: CodecPreference {
                allowed: vec![StreamCodec::H264, StreamCodec::H265, StreamCodec::Av1],
                preferred: None,
            },
            player: PlayerSettings {
                path: None,
                arguments: Vec::new(),
            },
            theme: Theme::System,
            language: "en".into(),
            notifications: NotificationSettings {
                live_channels: false,
                playback_errors: true,
            },
            hotkey: HotkeySettings {
                enabled: false,
                accelerator: "Ctrl+Shift+S".into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwitchUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub profile_image_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TwitchSession {
    Anonymous,
    Authenticated {
        user: TwitchUser,
        expires_at: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwitchLoginChallenge {
    pub verification_uri: String,
    pub user_code: String,
    pub expires_in_seconds: u32,
    pub polling_interval_seconds: u32,
}

#[cfg(test)]
mod tests {
    use super::{Settings, StreamCapabilities, TwitchSession};
    use serde::Deserialize;
    use serde_json::{Value, json};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        settings: Value,
        capabilities: Value,
        session: Value,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../../../fixtures/domain-contracts.json"))
            .expect("shared domain fixture must be valid")
    }

    #[test]
    fn shared_fixture_round_trips_in_rust() {
        let fixture = fixture();
        let settings: Settings = serde_json::from_value(fixture.settings.clone()).unwrap();
        let capabilities: StreamCapabilities =
            serde_json::from_value(fixture.capabilities.clone()).unwrap();
        let session: TwitchSession = serde_json::from_value(fixture.session.clone()).unwrap();

        assert_eq!(serde_json::to_value(settings).unwrap(), fixture.settings);
        let serialized_capabilities = serde_json::to_value(&capabilities).unwrap();
        let reparsed_capabilities: StreamCapabilities =
            serde_json::from_value(serialized_capabilities.clone()).unwrap();
        assert_eq!(reparsed_capabilities, capabilities);
        assert_eq!(serialized_capabilities["variants"][0]["fps"], json!(60.0));
        assert!(
            serialized_capabilities
                .get("futureStreamlinkFeature")
                .is_none()
        );
        assert_eq!(serde_json::to_value(session).unwrap(), fixture.session);
    }

    #[test]
    fn mutable_settings_reject_unknown_fields() {
        let mut settings = fixture().settings;
        settings["arbitraryCommand"] = json!("rm -rf /");

        let error = serde_json::from_value::<Settings>(settings).unwrap_err();
        assert!(error.to_string().contains("unknown field"));

        let mut nested_settings = fixture().settings;
        nested_settings["quality"]["futureLimit"] = json!(1);
        let nested_error = serde_json::from_value::<Settings>(nested_settings).unwrap_err();
        assert!(nested_error.to_string().contains("unknown field"));
    }

    #[test]
    fn read_only_capabilities_allow_unknown_fields() {
        let capabilities = serde_json::from_value::<StreamCapabilities>(fixture().capabilities);

        assert!(capabilities.is_ok());
    }
}
