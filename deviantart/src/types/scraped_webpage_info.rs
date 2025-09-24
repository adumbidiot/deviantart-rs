use super::Deviation;
use super::Media;
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

    /// Needed for login.
    ///
    /// Note that this is a different csrf token from the config struct.
    #[serde(rename = "csrfToken")]
    pub csrf_token: Option<Box<str>>,

    #[serde(rename = "gallectionSection")]
    pub gallection_section: Option<GallectionSection>,

    /// Needed for login.
    #[serde(rename = "luToken")]
    pub lu_token: Option<Box<str>>,

    /// Needed for login.
    #[serde(rename = "luToken2")]
    pub lu_token2: Option<Box<str>>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl ScrapedWebPageInfo {
    /// Parse this from a html string
    pub fn from_html_str(input: &str) -> Result<Self, FromHtmlStrError> {
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"window\.__INITIAL_STATE__ = JSON\.parse\("(.*)"\);"#).unwrap()
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

    /// Get the current folder id, if in a gallery.
    pub fn get_current_folder_id(&self) -> Option<u64> {
        Some(self.gallection_section.as_ref()?.selected_folder_id)
    }

    /// Get a stream for folder post ids, by folder id.
    ///
    /// This will return the deviation ids for the current folder.
    pub fn get_folder_deviations_stream(&self, folder_id: u64) -> Option<&WithOffsetStream> {
        let key = format!("folder-deviations-gallery-{folder_id}");

        self.streams
            .as_ref()?
            .streams
            .get(&key)?
            .as_with_offset_stream()
    }

    /// Get a gallery folder entity by id
    pub fn get_gallery_folder_entity(&self, folder_id: u64) -> Option<&GalleryFolder> {
        self.entities
            .as_ref()?
            .gallery_folder
            .as_ref()?
            .get(itoa::Buffer::new().format(folder_id))
    }

    /// Get a user entity by id
    pub fn get_user_entity(&self, user_id: u64) -> Option<&User> {
        self.entities
            .as_ref()?
            .user
            .as_ref()?
            .get(itoa::Buffer::new().format(user_id))
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

/// Page entities, like deviations, folders, and users.
#[derive(Debug, serde::Deserialize)]
pub struct Entities {
    /// Deviations
    pub deviation: HashMap<String, Deviation>,

    /// Extended Deviation Info
    #[serde(rename = "deviationExtended")]
    pub deviation_extended: Option<HashMap<String, DeviationExtended>>,

    /// Gallery folders
    #[serde(rename = "galleryFolder")]
    pub gallery_folder: Option<HashMap<String, GalleryFolder>>,

    /// Users
    pub user: Option<HashMap<String, User>>,

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

    /// Other media for this deviation
    #[serde(rename = "additionalMedia")]
    pub additional_media: Option<Vec<AdditionalMedia>>,

    /// Whether this image is protected.
    #[serde(rename = "isDaProtected")]
    pub is_da_protected: Option<bool>,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A gallery folder
#[derive(Debug, serde::Deserialize)]
pub struct GalleryFolder {
    /// The folder id.
    ///
    /// For some reason, this can be -1 sometimes.
    #[serde(rename = "folderId")]
    pub folder_id: i64,

    /// The name of the folder
    pub name: String,

    /// The user id of the owner of the folder
    pub owner: u64,

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A user
#[derive(Debug, serde::Deserialize)]
pub struct User {
    /// The user id
    #[serde(rename = "userId")]
    pub user_id: u64,

    /// The user name
    pub username: String,

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

#[derive(Debug, serde::Deserialize)]
pub struct AdditionalMedia {
    /// Media info
    pub media: Media,

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

    /// Extra data.
    ///
    /// This can include data whos purpose is known, like entries in a folder.
    #[serde(flatten)]
    pub streams: HashMap<String, Stream>,
}

/// ?
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "streamType")]
pub enum Stream {
    #[serde(rename = "WITH_OFFSET")]
    WithOffset(WithOffsetStream),

    #[serde(untagged)]
    Unknown(serde_json::Value),
}

impl Stream {
    /// Get this as a WithOffset stream.
    pub fn as_with_offset_stream(&self) -> Option<&WithOffsetStream> {
        match self {
            Self::WithOffset(stream) => Some(stream),
            _ => None,
        }
    }
}

/// ?
#[derive(Debug, serde::Deserialize)]
pub struct WithOffsetStream {
    /// Items in the stream?
    pub items: Vec<u64>,

    /// The # of items per fetch?
    #[serde(rename = "itemsPerFetch")]
    pub items_per_fetch: u32,

    /// Has more entries?
    #[serde(rename = "hasMore")]
    pub has_more: bool,

    /// ?
    #[serde(rename = "hasLess")]
    pub has_less: bool,

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

    /// Deviation ids?
    ///
    /// Usually, these are integers representing deviation ids.
    /// In some cases, these are strings of the format "xx-nnnnn",
    /// where the "xx" part is unknown and the "nnnnn" part is a deviation id.
    pub items: Vec<serde_json::Value>,

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

/// Gallery selection info
#[derive(Debug, serde::Deserialize)]
pub struct GallectionSection {
    /// The current page
    #[serde(rename = "currentPage")]
    pub page: u64,

    /// The id of the selected folder
    #[serde(rename = "selectedFolderId")]
    pub selected_folder_id: u64,

    /// The total number of pages
    #[serde(rename = "totalPages")]
    pub total_pages: u64,

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
