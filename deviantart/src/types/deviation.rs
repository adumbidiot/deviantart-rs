use super::Media;
use std::{collections::HashMap, path::Path};
use url::Url;

/// A Deviation
#[derive(Debug, serde::Deserialize)]
pub struct Deviation {
    // TODO: This is a number in a scraped deviation. Make either parse here.
    // /// DeviantArt Author
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
    pub media: Media,

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
            .append_pair("token", self.media.token.first()?);
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
        self.media.get_fullview_url()
    }

    /// Get the GIF url for this [`Deviation`].
    pub fn get_gif_url(&self) -> Option<Url> {
        let mut url = self.media.get_gif_media_type()?.b.clone()?;
        url.query_pairs_mut()
            .append_pair("token", self.media.token.first()?);
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
                .and_then(|media_type| media_type.b.as_ref())
                .or(self.media.base_uri.as_ref())?;
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

/// Text Content for literature
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Markup {
    pub version: u32,
    pub document: MarkupDocument,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MarkupDocument {
    pub content: Vec<MarkupDocumentContent>,

    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MarkupDocumentContent {
    #[serde(rename = "type")]
    pub kind: String,
    /// This is not just html element attributes,
    /// it also contains associated data only relavent for the given element.
    pub attrs: HashMap<String, serde_json::Value>,

    pub content: Option<Vec<MarkupDocumentContentInner>>,
}

/// This may be the same type as MarkupDocumentContent.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MarkupDocumentContentInner {
    #[serde(rename = "type")]
    pub kind: String,
    /// Only Some when kind is "text".
    pub text: Option<String>,
}
