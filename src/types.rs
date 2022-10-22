/// The `Deviation` type.
pub mod deviation;
/// The `OEmbed` type
pub mod oembed;
/// The `ScrapedStashInfo` type.
pub mod scraped_stash_info;
/// The `ScrapedWebPageInfo` type.
pub mod scraped_webpage_info;

pub use self::deviation::Deviation;
pub use self::oembed::OEmbed;
pub use self::scraped_stash_info::ScrapedStashInfo;
pub use self::scraped_webpage_info::DeviationExtended;
pub use self::scraped_webpage_info::ScrapedWebPageInfo;
