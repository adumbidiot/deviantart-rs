use super::Deviation;
use std::collections::HashMap;
use url::Url;

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

    /// Unknown data
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>,
}

impl ScrapedWebPageInfo {
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
        let mut key_buffer = itoa::Buffer::new();
        let key = match id {
            serde_json::Value::Number(n) => {
                let n = n.as_u64()?;
                key_buffer.format(n)
            }
            serde_json::Value::String(s) => s,
            _ => return None,
        };
        self.entities.as_ref()?.deviation.get(key)
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
