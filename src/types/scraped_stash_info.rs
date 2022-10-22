use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use url::Url;

/// An error that may occur while parsing a [`ScrapedStashInfo`] from a html str.
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlStrError {
    /// Missing the pageData variable
    #[error("missing pageData variable")]
    MissingPageData,

    /// Failed to parse json
    #[error(transparent)]
    InvalidJson(#[from] serde_json::Error),
}

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

impl ScrapedStashInfo {
    /// Parse this from a html str
    pub fn from_html_str(input: &str) -> Result<Self, FromHtmlStrError> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"deviantART.pageData=(.*);"#).expect("invalid `scrape_stash_info` regex")
        });

        let capture = REGEX
            .captures(input)
            .and_then(|captures| captures.get(1))
            .ok_or(FromHtmlStrError::MissingPageData)?;
        let scraped_stash: ScrapedStashInfo = serde_json::from_str(capture.as_str())?;

        Ok(scraped_stash)
    }
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
