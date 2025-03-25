use super::Deviation;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
pub struct ListFolderContentsResponse {
    /// Whether this has more
    #[serde(rename = "hasMore")]
    pub has_more: bool,

    /// The next offset
    #[serde(rename = "nextOffset")]
    pub next_offset: Option<u64>,

    /// results
    pub results: Vec<Deviation>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
