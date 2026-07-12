use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub login: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
    #[serde(alias = "profile_image_url")]
    pub profile_image_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub id: String,
    #[serde(alias = "user_id")]
    pub user_id: String,
    #[serde(alias = "user_login")]
    pub user_login: String,
    #[serde(alias = "user_name")]
    pub user_name: String,
    #[serde(alias = "game_id")]
    pub game_id: String,
    #[serde(alias = "game_name")]
    pub game_name: String,
    pub title: String,
    #[serde(alias = "viewer_count")]
    pub viewer_count: u64,
    #[serde(alias = "started_at")]
    pub started_at: String,
    #[serde(alias = "thumbnail_url")]
    pub thumbnail_url: String,
    #[serde(alias = "is_mature")]
    pub is_mature: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedChannel {
    #[serde(alias = "broadcaster_id")]
    pub broadcaster_id: String,
    #[serde(alias = "broadcaster_login")]
    pub broadcaster_login: String,
    #[serde(alias = "broadcaster_name")]
    pub broadcaster_name: String,
    #[serde(alias = "followed_at")]
    pub followed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    #[serde(alias = "box_art_url")]
    pub box_art_url: String,
    #[serde(default, alias = "igdb_id")]
    pub igdb_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchChannel {
    #[serde(alias = "broadcaster_language")]
    pub broadcaster_language: String,
    #[serde(alias = "broadcaster_login")]
    pub broadcaster_login: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
    #[serde(alias = "game_id")]
    pub game_id: String,
    #[serde(alias = "game_name")]
    pub game_name: String,
    pub id: String,
    #[serde(alias = "is_live")]
    pub is_live: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(alias = "thumbnail_url")]
    pub thumbnail_url: String,
    pub title: String,
    #[serde(alias = "started_at")]
    pub started_at: String,
}
