use std::collections::HashMap;
use url::Url;

/// DeviantArt [`DeviationMedia`] media type.
#[derive(Debug, serde::Deserialize)]
pub struct MediaType {
    /// The content. A uri used with base_uri.
    #[serde(rename = "c")]
    pub content: Option<String>,

    /// Image Height
    #[serde(rename = "h")]
    pub height: u64,

    // /// ?
    // // pub r: u64,
    /// The kind of media
    #[serde(rename = "t")]
    pub kind: String,

    /// Image Width
    #[serde(rename = "w")]
    pub width: u64,

    // /// ?
    // // pub f: Option<u64>,
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

/// A structure that stores media metadata.
///
/// Needed to create image urls.
#[derive(Debug, serde::Deserialize)]
pub struct Media {
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

impl Media {
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

    /// Get the fullview url for this [`Media`].
    pub fn get_fullview_url(&self) -> Option<Url> {
        let mut url = self.base_uri.as_ref()?.clone();

        // Allow the "content" section of the path to not exist, but the fullview data MUST exist.
        if let Some(path) = self.get_fullview_media_type()?.content.as_ref() {
            let mut path_segments_mut = url.path_segments_mut().ok()?;

            for path in path.split('/').filter(|p| !p.is_empty()) {
                // Replace "<pretty-name>" with the actual pretty name.
                let pretty_name = self.pretty_name.as_ref()?;
                let path = path.replace("<prettyName>", pretty_name);
                path_segments_mut.push(&path);
            }
        }

        // We assume that a token is not provided in cases where it is not needed.
        // As such, this part is optional.
        // So far, a token is allowed to be missing when the "content" section of the fullview data is missing
        // Correct this if these assumptions are wrong.
        if let Some(token) = self.token.first() {
            url.query_pairs_mut().append_pair("token", token);
        }

        Some(url)
    }
}
