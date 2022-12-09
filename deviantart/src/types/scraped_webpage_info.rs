use super::Deviation;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use url::Url;

/// An error that may occur while parsing a [`ScrapedWebPageInfo`] from a html string.
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlStrError {
    /// Missing the InitialState variable
    #[error("missing initial state")]
    MissingInitialState,

    /// Failed to parse some state
    #[error(transparent)]
    InvalidJson(#[from] serde_json::Error),
}

/// Info scraped from a deviation url
#[derive(Debug, serde::Deserialize)]
pub struct ScrapedWebPageInfo {
    /// Page config like csrf tokens
    #[serde(rename = "@@config")]
    pub config: Config,

    /// Deviations extended deviations maybe?
    #[serde(rename = "@@entities")]
    pub entities: Option<Entities>,

    /// ?
    #[serde(rename = "@@DUPERBROWSE")]
    pub duper_browse: Option<DuperBrowse>,

    /// Info about the current session
    #[serde(rename = "@@publicSession")]
    pub public_session: PublicSession,

    /// Streams
    #[serde(rename = "@@streams")]
    pub streams: Option<Streams>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl ScrapedWebPageInfo {
    /// Parse this from a html string
    pub fn from_html_str(input: &str) -> Result<Self, FromHtmlStrError> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"window\.__INITIAL_STATE__ = JSON\.parse\("(.*)"\);"#)
                .expect("invalid `scrape_deviation` regex")
        });

        let capture = REGEX
            .captures(input)
            .and_then(|captures| captures.get(1))
            .ok_or(FromHtmlStrError::MissingInitialState)?;
        // TODO: Escape properly
        let capture = capture
            .as_str()
            .replace("\\\"", "\"")
            .replace("\\'", "'")
            .replace("\\\\", "\\");
        Ok(serde_json::from_str(&capture)?)
    }

    /// Returns `true` if logged in
    pub fn is_logged_in(&self) -> bool {
        self.public_session.is_logged_in
    }

    /// Get the current deviation's id
    pub fn get_current_deviation_id(&self) -> Option<&serde_json::Value> {
        Some(
            &self
                .duper_browse
                .as_ref()?
                .root_stream
                .as_ref()?
                .current_open_item,
        )
    }

    /// Get the [`Deviation`] for this page.
    pub fn get_current_deviation(&self) -> Option<&Deviation> {
        let id = self.get_current_deviation_id()?;
        let id = match id {
            serde_json::Value::Number(n) => n.as_u64()?,
            serde_json::Value::String(s) => s.parse().ok()?,
            _ => return None,
        };
        self.get_deviation_by_id(id)
    }

    /// Get the [`DeviationExtended`] for this page.
    pub fn get_current_deviation_extended(&self) -> Option<&DeviationExtended> {
        let id = self.get_current_deviation_id()?;
        let mut key_buffer = itoa::Buffer::new();
        let key = match id {
            serde_json::Value::Number(n) => {
                let n = n.as_u64()?;
                key_buffer.format(n)
            }
            serde_json::Value::String(s) => s,
            _ => return None,
        };
        self.entities
            .as_ref()?
            .deviation_extended
            .as_ref()?
            .get(key)
    }

    /// Get a deviation by id, if it exists
    pub fn get_deviation_by_id(&self, id: u64) -> Option<&Deviation> {
        let mut key_buffer = itoa::Buffer::new();
        self.entities.as_ref()?.deviation.get(key_buffer.format(id))
    }

    /// Take a deviation by id, if it exists
    pub fn take_deviation_by_id(&mut self, id: u64) -> Option<Deviation> {
        let mut key_buffer = itoa::Buffer::new();
        self.entities
            .as_mut()?
            .deviation
            .remove(key_buffer.format(id))
    }
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// The page's csrf token
    #[serde(rename = "csrfToken")]
    pub csrf_token: String,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct Entities {
    /// Deviations
    pub deviation: HashMap<String, Deviation>,

    /// Extended Deviation Info
    #[serde(rename = "deviationExtended")]
    pub deviation_extended: Option<HashMap<String, DeviationExtended>>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Extended Info about a deviation
#[derive(Debug, serde::Deserialize)]
pub struct DeviationExtended {
    /// Download info
    pub download: Option<Download>,

    /// HTML description
    pub description: Option<String>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Download {
    /// The file size
    pub filesize: u64,

    /// The image height
    pub height: u32,

    /// The image width
    pub width: u32,

    /// ?
    #[serde(rename = "type")]
    pub kind: String,

    /// The url
    pub url: Url,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct DuperBrowse {
    /// ?
    #[serde(rename = "rootStream")]
    pub root_stream: Option<RootStream>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct RootStream {
    /// The id of the current deviation. This is either a number or string.
    #[serde(rename = "currentOpenItem")]
    pub current_open_item: serde_json::Value,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct PublicSession {
    /// Whether the user is logged in
    #[serde(rename = "isLoggedIn")]
    pub is_logged_in: bool,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// The streams field
#[derive(Debug, serde::Deserialize)]
pub struct Streams {
    /// Search results appear here
    #[serde(rename = "@@BROWSE_PAGE_STREAM")]
    pub browse_page_stream: Option<BrowsePageStream>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Search results appear here
#[derive(Debug, serde::Deserialize)]
pub struct BrowsePageStream {
    /// The cursor
    pub cursor: String,

    /// Whether this has less?
    #[serde(rename = "hasLess")]
    pub has_less: bool,

    /// Whether this has more?
    #[serde(rename = "hasMore")]
    pub has_more: bool,

    /// deviation ids
    pub items: Vec<u64>,

    /// The # of items per page
    #[serde(rename = "itemsPerFetch")]
    pub items_per_fetch: u64,

    /// Stream Params
    #[serde(rename = "streamParams")]
    pub stream_params: StreamParams,

    /// The stream type
    #[serde(rename = "streamType")]
    pub stream_type: String,

    /// The stream id
    #[serde(rename = "streamId")]
    pub stream_id: String,

    /// ?
    #[serde(rename = "fetchNextCallback")]
    pub fetch_next_callback: String,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Stream params
#[derive(Debug, serde::Deserialize)]
pub struct StreamParams {
    /// Request params
    #[serde(rename = "requestParams")]
    pub request_params: HashMap<String, String>,

    /// ?
    #[serde(rename = "itemType")]
    pub item_type: String,

    /// ?
    #[serde(rename = "requestEndpoint")]
    pub request_endpoint: String,

    /// ?
    #[serde(rename = "initialOffset")]
    pub initial_offset: u64,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;

    const SCRAPED_WEBPAGE: &str = include_str!("../../test_data/scraped_webpage.json");
    const LOGIN_WEBPAGE: &str = include_str!("../../test_data/login_webpage.json");

    #[test]
    fn parse_scraped_webpage() {
        let scraped_webpage_info: ScrapedWebPageInfo =
            serde_json::from_str(SCRAPED_WEBPAGE).expect("failed to parse scraped webpage info");
        assert_eq!(
            scraped_webpage_info
                .get_current_deviation_id()
                .expect("missing current deviation id"),
            119577071
        );
        // dbg!(scraped_deviation_info.entities.deviation);
    }

    #[test]
    fn parse_login_webpage() {
        let _scraped_webpage_info: ScrapedWebPageInfo =
            serde_json::from_str(LOGIN_WEBPAGE).expect("failed to parse scraped webpage info");
    }
}
