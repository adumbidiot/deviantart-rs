/// The client
mod client;
/// API types
pub mod types;

pub use self::client::Client;
pub use self::types::Deviation;
pub use self::types::OEmbed;
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
}

// TODO:
// investigate deviantart.com/view/<id>
// ex: deviantart.com/view/852625718
