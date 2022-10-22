use crate::Deviation;
use crate::Error;
use crate::OEmbed;
use crate::ScrapedStashInfo;
use crate::ScrapedWebPageInfo;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_cookie_store::CookieStoreMutex;
use std::sync::Arc;
use url::Url;

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36";

/// A DeviantArt Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client. You probably shouldn't touch this.
    pub client: reqwest::Client,

    /// The cookie store.
    pub cookie_store: Arc<CookieStoreMutex>,
}

impl Client {
    /// Make a new [`Client`].
    pub fn new() -> Self {
        Self::new_with_user_agent(USER_AGENT_STR)
    }

    /// Make a new [`Client`] with the given user agent.
    pub fn new_with_user_agent(user_agent: &str) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en,en-US;q=0,5"),
        );
        default_headers.insert(reqwest::header::ACCEPT, HeaderValue::from_static("*/*"));
        default_headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static("https://www.deviantart.com/"),
        );

        let cookie_store = Arc::new(CookieStoreMutex::new(Default::default()));
        let client = reqwest::Client::builder()
            .cookie_provider(cookie_store.clone())
            .user_agent(user_agent)
            .default_headers(default_headers)
            .build()
            .expect("failed to build deviantart client");

        Client {
            client,
            cookie_store,
        }
    }

    /// Scrape a webpage for info.
    pub async fn scrape_webpage(&self, url: &str) -> Result<ScrapedWebPageInfo, Error> {
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let scraped_webpage =
            tokio::task::spawn_blocking(move || ScrapedWebPageInfo::from_html_str(&text)).await??;

        Ok(scraped_webpage)
    }

    /// Sign in to get access to more results from apis.
    ///
    /// This will also clean the cookie jar.
    pub async fn sign_in(&self, username: &str, password: &str) -> Result<(), Error> {
        // Clean the jar of expired cookies
        {
            let mut cookie_store = self.cookie_store.lock().expect("cookie store is poisoned");

            // We need to allocate here as the cookie_store iter cannot be alive when we try to remove items from the cookie store.
            let to_remove: Vec<_> = cookie_store
                .iter_any()
                .filter(|cookie| cookie.is_expired())
                .map(|cookie| {
                    let domain = cookie.domain().unwrap_or("");
                    let name = cookie.name();
                    let path = cookie.path().unwrap_or("");

                    (domain.to_string(), name.to_string(), path.to_string())
                })
                .collect();

            for (domain, name, path) in to_remove {
                cookie_store.remove(&domain, &name, &path);
            }
        }

        let scraped_webpage = self
            .scrape_webpage("https://www.deviantart.com/users/login")
            .await?;
        let res = self
            .client
            .post("https://www.deviantart.com/_sisu/do/signin")
            .form(&[
                ("referer", "https://www.deviantart.com/"),
                ("csrf_token", &scraped_webpage.config.csrf_token),
                ("username", username),
                ("password", password),
                ("challenge", "0"),
                ("remember", "on"),
            ])
            .send()
            .await?
            .error_for_status()?;

        let text = res.text().await?;
        let scraped_webpage =
            tokio::task::spawn_blocking(move || ScrapedWebPageInfo::from_html_str(&text)).await??;
        if !scraped_webpage.is_logged_in() {
            return Err(Error::SignInFailed);
        }

        Ok(())
    }

    /// Run a GET request on the home page and check if the user is logged in
    pub async fn is_logged_in_online(&self) -> Result<bool, Error> {
        Ok(self
            .scrape_webpage("https://www.deviantart.com/")
            .await?
            .is_logged_in())
    }

    /// OEmbed API
    pub async fn get_oembed(&self, url: &str) -> Result<OEmbed, Error> {
        let url = Url::parse_with_params("https://backend.deviantart.com/oembed", &[("url", url)])?;
        let res = self
            .client
            .get(url.as_str())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    /// Scrape a sta.sh link for info
    pub async fn scrape_stash_info(&self, url: &str) -> Result<ScrapedStashInfo, Error> {
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let scraped_stash =
            tokio::task::spawn_blocking(move || ScrapedStashInfo::from_html_str(&text)).await??;

        Ok(scraped_stash)
    }

    /// Run a search using the low level api
    pub async fn search_raw(
        &self,
        query: &str,
        cursor: Option<&str>,
    ) -> Result<ScrapedWebPageInfo, Error> {
        let mut url = Url::parse_with_params("https://www.deviantart.com/search", &[("q", query)])?;
        {
            let mut query_pairs = url.query_pairs_mut();
            if let Some(cursor) = cursor {
                query_pairs.append_pair("cursor", cursor);
            }
        }

        self.scrape_webpage(url.as_str()).await
    }

    /// Run a search
    pub fn search(&self, query: &str, cursor: Option<&str>) -> SearchCursor {
        SearchCursor::new(self.clone(), query, cursor)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SearchCursor {
    /// The client
    client: Client,

    /// The current page
    page: Option<ScrapedWebPageInfo>,

    /// the query
    query: String,
    /// the cursor
    cursor: Option<String>,
}

impl SearchCursor {
    /// Make a new Search Cursor
    pub fn new(client: Client, query: &str, cursor: Option<&str>) -> Self {
        Self {
            client,

            page: None,

            query: query.into(),
            cursor: cursor.map(|c| c.into()),
        }
    }

    /// Get the next page, updating the internal cursor.
    pub async fn next_page(&mut self) -> Result<Vec<&Deviation>, Error> {
        let page = self
            .client
            .search_raw(&self.query, self.cursor.as_deref())
            .await?;
        // Validate before storing
        if page
            .streams
            .as_ref()
            .ok_or(Error::MissingStreams)?
            .browse_page_stream
            .is_none()
        {
            return Err(Error::MissingBrowsePageStream);
        }
        self.page = Some(page);
        let page = self.page.as_ref().unwrap();

        let browse_page_stream = page
            .streams
            .as_ref()
            .unwrap()
            .browse_page_stream
            .as_ref()
            .unwrap();
        self.cursor = Some(browse_page_stream.cursor.clone());

        browse_page_stream
            .items
            .iter()
            .map(|id| {
                page.get_deviation_by_id(*id)
                    .ok_or_else(|| Error::MissingDeviation(*id))
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Config {
        username: String,
        password: String,
    }

    impl Config {
        fn from_path(path: &str) -> Config {
            let file = std::fs::read(path).expect("failed to read config");
            serde_json::from_reader(file.as_slice()).expect("failed to parse config")
        }
    }

    #[tokio::test]
    async fn scrape_deviation() {
        let client = Client::new();
        let _scraped_webpage = client
            .scrape_webpage("https://www.deviantart.com/zilla774/art/chaos-gerbil-RAWR-119577071")
            .await
            .expect("failed to scrape webpage");
    }

    #[tokio::test]
    #[ignore]
    async fn sign_in_works() {
        let config: Config = Config::from_path("config.json");

        let client = Client::new();
        client
            .sign_in(&config.username, &config.password)
            .await
            .expect("failed to sign in");
        let is_online = client
            .is_logged_in_online()
            .await
            .expect("failed to check if online");
        assert!(is_online);
    }

    #[tokio::test]
    async fn scrape_webpage_literature() {
        let client = Client::new();
        let scraped_webpage = client
            .scrape_webpage("https://www.deviantart.com/tohokari-steel/art/A-Fictorian-Tale-Chapter-11-879180914")
            .await
            .expect("failed to scrape webpage");
        let current_deviation = scraped_webpage
            .get_current_deviation()
            .expect("missing current deviation");
        let text_content = current_deviation
            .text_content
            .as_ref()
            .expect("missing text content");
        let _markup = text_content
            .html
            .get_markup()
            .expect("missing markup")
            .expect("failed to parse markup");
        // dbg!(&markup);
    }

    #[tokio::test]
    async fn oembed_works() {
        let client = Client::new();
        let oembed = client.get_oembed("https://www.deviantart.com/tohokari-steel/art/A-Fictorian-Tale-Chapter-11-879180914").await.expect("failed to get oembed");
        assert!(oembed.title == "A Fictorian Tale Chapter 11");
    }

    #[tokio::test]
    async fn scrape_stash_info_works() {
        let client = Client::new();
        let url = "https://sta.sh/02bhirtp3iwq";
        let stash = client
            .scrape_stash_info(url)
            .await
            .expect("failed to scrape stash");
        assert!(stash.deviationid == 590293385);
    }

    #[tokio::test]
    async fn it_works() {
        let client = Client::new();
        let mut cursor = client.search("sun", None);
        let next_page = cursor.next_page().await.expect("failed to get next page");
        dbg!(next_page);
    }
}
