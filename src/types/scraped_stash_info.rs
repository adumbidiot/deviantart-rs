use std::collections::HashMap;
use url::Url;

/// Scraped info from a sta.sh link
#[derive(Debug, serde::Deserialize)]
pub struct ScrapedStashInfo {
    /// Csrf token
    pub csrf: String,

    /// ?
    pub deviationid: u64,

    /// Present only if it is a video
    pub film: Option<Film>,

    /// The width
    pub deviation_width: u64,

    /// The height
    pub deviation_height: u64,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Film data from a sta.sh link
#[derive(Debug, serde::Deserialize)]
pub struct Film {
    /// Video sizes
    pub sizes: HashMap<String, Size>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Film {
    /// Get the best size
    pub fn get_best_size(&self) -> Option<&Size> {
        self.sizes.values().max_by_key(|v| v.width * v.height)
    }
}

/// Film size
#[derive(Debug, serde::Deserialize)]
pub struct Size {
    /// Video height
    pub height: u32,

    /// Video width
    pub width: u32,

    /// Video src
    pub src: Url,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
