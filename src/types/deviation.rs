use std::{collections::HashMap, path::Path};
use url::Url;

/// A Deviation
#[derive(Debug, serde::Deserialize)]
pub struct Deviation {
    // TODO: This is a number in a scraped deviation. Make either parse here.
    /// DeviantArt Author
    // pub author: Author,

    /// ?
    #[serde(rename = "blockReasons")]
    pub block_reasons: Vec<serde_json::Value>,

    /// Deviation ID
    #[serde(rename = "deviationId")]
    pub deviation_id: u64,

    /// Deviation Type
    #[serde(rename = "type")]
    pub kind: String,

    /// Image Url
    pub url: Url,

    /// Media Info
    pub media: DeviationMedia,

    /// Title
    pub title: String,

    /// Text content for literature
    #[serde(rename = "textContent")]
    pub text_content: Option<TextContext>,

    /// Whether this is downloadable
    #[serde(rename = "isDownloadable")]
    pub is_downloadable: bool,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Deviation {
    /// Get the media url for this [`Deviation`].
    pub fn get_media_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(0)?);
        Some(url)
    }

    /// Get the "download" url for this [`Deviation`].
    pub fn get_download_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(1)?);
        Some(url)
    }

    /// Get the fullview url for this [`Deviation`].
    pub fn get_fullview_url(&self) -> Option<Url> {
        let mut url = self.media.base_uri.as_ref()?.clone();

        // Allow the "content" section of the path to not exist, but the fullview data MUST exist.
        if let Some(path) = self.media.get_fullview_media_type()?.content.as_ref() {
            let mut path_segments_mut = url.path_segments_mut().ok()?;

            for path in path.split('/').filter(|p| !p.is_empty()) {
                // Replace "<pretty-name>" with the actual pretty name.
                let pretty_name = self.media.pretty_name.as_ref()?;
                let path = path.replace("<prettyName>", pretty_name);
                path_segments_mut.push(&path);
            }
        }

        // We assume that a token is not provided in cases where it is not needed.
        // As such, this part is optional.
        // So far, a token is allowed to be missing when the "content" section of the fullview data is missing
        // Correct this if these assumptions are wrong.
        if let Some(token) = self.media.token.get(0) {
            url.query_pairs_mut().append_pair("token", token);
        }

        Some(url)
    }

    /// Get the GIF url for this [`Deviation`].
    pub fn get_gif_url(&self) -> Option<Url> {
        let mut url = self.media.get_gif_media_type()?.b.clone()?;
        url.query_pairs_mut()
            .append_pair("token", self.media.token.get(0)?);
        Some(url)
    }

    /// Get the best video url
    pub fn get_best_video_url(&self) -> Option<&Url> {
        let url = self.media.get_best_video_media_type()?.b.as_ref()?;
        Some(url)
    }

    /// Whether this is an image
    pub fn is_image(&self) -> bool {
        self.kind == "image"
    }

    /// Whether this is literature
    pub fn is_literature(&self) -> bool {
        self.kind == "literature"
    }

    /// Whether this is a film
    pub fn is_film(&self) -> bool {
        self.kind == "film"
    }

    /// Get the most "fitting" url to download an image.
    ///
    /// Usually, [`DeviationExtended`] holds better data than a [`Deviation`], so prefer that instead.
    pub fn get_image_download_url(&self) -> Option<Url> {
        // Try to get the download url.
        if let Some(url) = self.get_download_url() {
            return Some(url);
        }

        // If that fails, this is probably a gif, so we try to get the gif url.
        if let Some(url) = self.get_gif_url() {
            return Some(url);
        }

        // Otherwise, assume failure
        None
    }
    /// Try to get the original extension of this [`Deviation`]
    pub fn get_extension(&self) -> Option<&str> {
        if self.is_image() {
            let url = self
                .media
                .get_gif_media_type()
                .and_then(|media_type| media_type.b.as_ref().or(self.media.base_uri.as_ref()))?;
            Path::new(url.as_str()).extension()?.to_str()
        } else if self.is_literature() {
            None
        } else if self.is_film() {
            let url = self.media.get_best_video_media_type()?.b.as_ref()?;
            Path::new(url.as_str()).extension()?.to_str()
        } else {
            None
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Author {
    /// is the user new
    #[serde(rename = "isNewDeviant")]
    pub is_new_deviant: bool,

    /// User UUID
    #[serde(rename = "useridUuid")]
    pub userid_uuid: String,

    /// User icon url
    pub usericon: Url,

    /// User ID
    #[serde(rename = "userId")]
    pub user_id: u64,

    /// Username
    pub username: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// The media field of a [`Deviation`].
#[derive(Debug, serde::Deserialize)]
pub struct DeviationMedia {
    /// The base uri
    #[serde(rename = "baseUri")]
    pub base_uri: Option<Url>,

    /// Image token
    #[serde(default)]
    pub token: Vec<String>,

    /// Types
    pub types: Vec<MediaType>,

    /// Pretty Name
    #[serde(rename = "prettyName")]
    pub pretty_name: Option<String>,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl DeviationMedia {
    /// Try to get the fullview [`MediaType`].
    pub fn get_fullview_media_type(&self) -> Option<&MediaType> {
        self.types.iter().find(|t| t.is_fullview())
    }

    /// Try to get the gif [`MediaType`].
    pub fn get_gif_media_type(&self) -> Option<&MediaType> {
        self.types.iter().find(|t| t.is_gif())
    }

    /// Try to get the video [`MediaType`]
    pub fn get_best_video_media_type(&self) -> Option<&MediaType> {
        self.types
            .iter()
            .filter(|media_type| media_type.is_video())
            .max_by_key(|media_type| media_type.width)
    }
}

/// DeviantArt [`DeviationMedia`] media type.
#[derive(Debug, serde::Deserialize)]
pub struct MediaType {
    /// The content. A uri used with base_uri.
    #[serde(rename = "c")]
    pub content: Option<String>,

    /// Image Height
    #[serde(rename = "h")]
    pub height: u64,

    /// ?
    // pub r: u64,

    /// The kind of media
    #[serde(rename = "t")]
    pub kind: String,

    /// Image Width
    #[serde(rename = "w")]
    pub width: u64,

    /// ?
    // pub f: Option<u64>,

    /// ?
    pub b: Option<Url>,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl MediaType {
    /// Whether this is the fullview
    pub fn is_fullview(&self) -> bool {
        self.kind == "fullview"
    }

    /// Whether this is a gif
    pub fn is_gif(&self) -> bool {
        self.kind == "gif"
    }

    /// Whether this is a video
    pub fn is_video(&self) -> bool {
        self.kind == "video"
    }
}

/// Text Content for literature
#[derive(Debug, serde::Deserialize)]
pub struct TextContext {
    /// Excerpt of text
    pub excerpt: String,

    /// Html data
    pub html: Html,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// Text Context html
#[derive(Debug, serde::Deserialize)]
pub struct Html {
    /// ?
    pub features: String,

    /// Text markup data
    pub markup: Option<String>,

    /// The kind of text data
    #[serde(rename = "type")]
    pub kind: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl Html {
    /// Try to parse the markup field
    pub fn get_markup(&self) -> Option<Result<Markup, serde_json::Error>> {
        let markup = self.markup.as_ref()?;
        Some(serde_json::from_str(markup))
    }
}

/// Text Context Html Markup
#[derive(Debug, serde::Deserialize)]
pub struct Markup {
    /// Blocks of marked-up text
    pub blocks: Vec<Block>,

    /// ?
    #[serde(rename = "entityMap")]
    pub entity_map: serde_json::Value,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

/// A Markup block
#[derive(Debug, serde::Deserialize)]
pub struct Block {
    /// ?
    pub data: serde_json::Value,

    /// ?
    pub depth: u64,

    /// ?
    pub key: String,

    /// Text data
    pub text: String,

    #[serde(rename = "type")]
    pub kind: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
