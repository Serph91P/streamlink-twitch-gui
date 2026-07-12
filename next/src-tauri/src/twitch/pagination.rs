use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at_epoch_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WirePage<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub pagination: WirePagination,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct WirePagination {
    pub cursor: Option<String>,
}
