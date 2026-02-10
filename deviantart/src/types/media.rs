use std::collections::HashMap;
use std::fmt::Write;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum GetFullviewUrlError {
    #[error("missing base uri")]
    MissingBaseUri,
    #[error("missing media type")]
    MissingMediaType,
    #[error("unexpected path part \"{path_part}\"")]
    UnexpectedPathPart { path_part: String },
    #[error("expected {expected}, found \"{actual}\"")]
    InvalidPathPart {
        actual: String,
        expected: &'static str,
    },
    #[error("missing path part")]
    MissingPathPart,
    #[error("missing pretty name")]
    MissingPrettyName,
    #[error("missing pretty name template")]
    MissingPrettyNameTemplate,
    #[error("invalid fullview option format")]
    InvalidFullviewOptionFormat,
    #[error("invalid fullview option \"{option}\"")]
    InvalidFullviewOption { option: String },
    #[error("duplicate fullview option \"{option}\"")]
    DuplicateFullviewOption { option: String },
    #[error("the fullview url is not for a jpg")]
    NotJpg,
}

#[derive(Debug, Clone)]
pub struct GetFullviewUrlOptions {
    pub quality: Option<u8>,
    pub strp: Option<bool>,
    pub png: bool,
}

impl GetFullviewUrlOptions {
    pub fn new() -> Self {
        Self {
            quality: None,
            strp: None,
            png: false,
        }
    }
}

impl Default for GetFullviewUrlOptions {
    fn default() -> Self {
        Self::new()
    }
}

fn create_fullview_path(
    path: &str,
    pretty_name: &str,
    options: GetFullviewUrlOptions,
    path_segments_mut: &mut url::PathSegmentsMut,
) -> Result<(), GetFullviewUrlError> {
    // Parse: /v1/{fill,fit}/w_1280,h_1024,q_80,strp/<prettyName>-fullview.jpg
    let mut path_iter = path.split('/').filter(|p| !p.is_empty());

    {
        let path_part = path_iter
            .next()
            .ok_or(GetFullviewUrlError::MissingPathPart)?;
        if path_part != "v1" {
            return Err(GetFullviewUrlError::InvalidPathPart {
                actual: path_part.to_string(),
                expected: "v1",
            });
        }
        path_segments_mut.push(path_part);
    }

    {
        let path_part = path_iter
            .next()
            .ok_or(GetFullviewUrlError::MissingPathPart)?;
        if path_part != "fit" && path_part != "fill" {
            return Err(GetFullviewUrlError::InvalidPathPart {
                actual: path_part.to_string(),
                expected: "\"fit\" or \"fill\"",
            });
        }
        path_segments_mut.push(path_part);
    }

    {
        let path_part = path_iter
            .next()
            .ok_or(GetFullviewUrlError::MissingPathPart)?;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut quality: Option<u8> = None;
        let mut strp = false;
        for part in path_part.split(",") {
            if part == "strp" {
                strp = true;
                continue;
            }

            let (name, value) = part
                .split_once('_')
                .ok_or(GetFullviewUrlError::InvalidFullviewOptionFormat)?;
            match name {
                "w" => {
                    if width.is_some() {
                        return Err(GetFullviewUrlError::DuplicateFullviewOption {
                            option: name.to_string(),
                        });
                    }
                    width = Some(value.parse().map_err(|_err| {
                        GetFullviewUrlError::InvalidFullviewOption {
                            option: name.to_string(),
                        }
                    })?);
                }
                "h" => {
                    if height.is_some() {
                        return Err(GetFullviewUrlError::DuplicateFullviewOption {
                            option: name.to_string(),
                        });
                    }
                    height = Some(value.parse().map_err(|_err| {
                        GetFullviewUrlError::InvalidFullviewOption {
                            option: name.to_string(),
                        }
                    })?);
                }
                "q" => {
                    if quality.is_some() {
                        return Err(GetFullviewUrlError::DuplicateFullviewOption {
                            option: name.to_string(),
                        });
                    }
                    quality = Some(value.parse().map_err(|_err| {
                        GetFullviewUrlError::InvalidFullviewOption {
                            option: name.to_string(),
                        }
                    })?);
                }
                _ => {
                    return Err(GetFullviewUrlError::InvalidFullviewOption {
                        option: name.to_string(),
                    });
                }
            }
        }
        let mut new_path_part = String::new();
        if let Some(width) = width {
            write!(&mut new_path_part, "w_{width}").unwrap();
        }
        if let Some(height) = height {
            if !new_path_part.is_empty() {
                new_path_part.push(',');
            }
            write!(&mut new_path_part, "h_{height}").unwrap();
        }
        if let Some(quality) = options.quality.or(quality) {
            if !new_path_part.is_empty() {
                new_path_part.push(',');
            }
            write!(&mut new_path_part, "q_{quality}").unwrap();
        }
        if options.strp.unwrap_or(strp) {
            if !new_path_part.is_empty() {
                new_path_part.push(',');
            }
            new_path_part.push_str("strp");
        }
        path_segments_mut.push(&new_path_part);
    }

    {
        const TEMPLATE: &str = "<prettyName>";

        let path_part = path_iter
            .next()
            .ok_or(GetFullviewUrlError::MissingPathPart)?;
        if !path_part.contains(TEMPLATE) {
            return Err(GetFullviewUrlError::MissingPrettyNameTemplate);
        }
        // Replace "<pretty-name>" with the actual pretty name.
        let mut path_part = path_part.replace(TEMPLATE, pretty_name);

        // Make the url a png url if requested.
        if options.png {
            let path_part_stem = path_part
                .strip_suffix(".jpg")
                .ok_or(GetFullviewUrlError::NotJpg)?;
            path_part = format!("{path_part_stem}.png");
        }

        path_segments_mut.push(&path_part);
    }

    {
        let path_part = path_iter.next();
        if let Some(path_part) = path_part {
            return Err(GetFullviewUrlError::UnexpectedPathPart {
                path_part: path_part.to_string(),
            });
        }
    }

    Ok(())
}

/// DeviantArt [`DeviationMedia`] media type.
#[derive(Debug, serde::Deserialize)]
pub struct MediaType {
    /// The content. A uri used with base_uri.
    #[serde(rename = "c")]
    pub content: Option<String>,

    /// Image height
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
    pub fn get_fullview_url(
        &self,
        options: GetFullviewUrlOptions,
    ) -> Result<Url, GetFullviewUrlError> {
        let mut url = self
            .base_uri
            .as_ref()
            .ok_or(GetFullviewUrlError::MissingBaseUri)?
            .clone();

        // Allow the "content" section of the path to not exist, but the fullview data MUST exist.
        if let Some(path) = self
            .get_fullview_media_type()
            .ok_or(GetFullviewUrlError::MissingMediaType)?
            .content
            .as_ref()
        {
            let mut path_segments_mut = url
                .path_segments_mut()
                .ok()
                .ok_or(GetFullviewUrlError::MissingPathPart)?;

            let pretty_name = self
                .pretty_name
                .as_ref()
                .ok_or(GetFullviewUrlError::MissingPrettyName)?;
            create_fullview_path(path, pretty_name, options, &mut path_segments_mut)?;
        }

        // We assume that a token is not provided in cases where it is not needed.
        // As such, this part is optional.
        // So far, a token is allowed to be missing when the "content" section of the fullview data is missing
        // Correct this if these assumptions are wrong.
        if let Some(token) = self.token.first() {
            url.query_pairs_mut().append_pair("token", token);
        }

        Ok(url)
    }
}
