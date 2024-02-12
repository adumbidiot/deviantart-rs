/// The client
mod client;
/// API types
pub mod types;

pub use self::client::Client;
pub use self::types::Deviation;
pub use self::types::DeviationExtended;
pub use self::types::OEmbed;
pub use self::types::ScrapedStashInfo;
pub use self::types::ScrapedWebPageInfo;

/// Library Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Invalid Url
    #[error(transparent)]
    Url(#[from] url::ParseError),

    /// A tokio task failed to join
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Json failed to parse
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// A scraped web page was invalid
    #[error("invalid scraped webpage")]
    InvalidScrapedWebPage(#[from] self::types::scraped_webpage_info::FromHtmlStrError),

    /// Scraped Stash info was invalid
    #[error("invalid scraped stash info")]
    InvalidScrapedStashInfo(#[from] self::types::scraped_stash_info::FromHtmlStrError),

    /// Signing in failed for an unspecified reason
    #[error("sign in failed")]
    SignInFailed,

    /// Missing a field
    #[error("missing field \"{name}\"")]
    MissingField {
        /// The missing field name
        name: &'static str,
    },

    /// Missing the streams field
    #[error("missing streams")]
    MissingStreams,

    /// Missing the browse page stream
    #[error("missing browse page stream")]
    MissingBrowsePageStream,

    /// Missing the Deviation of the given id
    #[error("missing deviation {0}")]
    MissingDeviation(u64),

    /// A cookie store error occured
    #[error("cookie store error")]
    CookieStore(WrapBoxError),
}

/// A wrapper over a `Box<dyn std::error::Error + Send + Sync + 'static>`.
#[derive(Debug)]
pub struct WrapBoxError(pub Box<dyn std::error::Error + Send + Sync + 'static>);

impl std::fmt::Display for WrapBoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for WrapBoxError {}

// TODO:
// investigate deviantart.com/view/<id>
// ex: deviantart.com/view/852625718
