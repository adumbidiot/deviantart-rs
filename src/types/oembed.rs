use std::collections::HashMap;
 use url::Url;

/// DeviantArt OEmbed
#[derive(Debug, serde::Deserialize)]
pub struct OEmbed {
    /// Url of the asset
    pub url: Url,

    /// Url of the thumbnail
    pub thumbnail_url: Option<Url>,

    /// Title
    pub title: String,

    /// Unknown K/Vs
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}
