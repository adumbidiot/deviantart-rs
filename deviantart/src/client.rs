use crate::Deviation;
use crate::Error;
use crate::OEmbed;
use crate::ScrapedWebPageInfo;
use crate::WrapBoxError;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_cookie_store::CookieStoreMutex;
use std::sync::Arc;
use url::Url;

const USER_AGENT_STR: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36";
static ACCEPT_LANGUAGE_VALUE: HeaderValue = HeaderValue::from_static("en,en-US;q=0,5");
static ACCEPT_VALUE: HeaderValue = HeaderValue::from_static("*/*");
static REFERER_VALUE: HeaderValue = HeaderValue::from_static(HOME_URL);

const HOME_URL: &str = "https://www.deviantart.com/";
const LOGIN_URL: &str = "https://www.deviantart.com/users/login";

/// A DeviantArt Client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client.
    ///
    /// You probably shouldn't touch this.
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
            ACCEPT_LANGUAGE_VALUE.clone(),
        );
        default_headers.insert(reqwest::header::ACCEPT, ACCEPT_VALUE.clone());
        default_headers.insert(reqwest::header::REFERER, REFERER_VALUE.clone());

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

    /// Load the cookie store from a json reader.
    pub async fn load_json_cookies<R>(&self, reader: R) -> Result<(), Error>
    where
        R: std::io::BufRead + Send + 'static,
    {
        let cookie_store = self.cookie_store.clone();
        tokio::task::spawn_blocking(move || {
            let new_cookie_store = cookie_store::serde::json::load(reader)
                .map_err(|e| Error::CookieStore(WrapBoxError(e)))?;
            let mut cookie_store = cookie_store.lock().expect("cookie store is poisoned");
            *cookie_store = new_cookie_store;
            Ok(())
        })
        .await?
    }

    /// Save the cookie store from a json writer.
    pub async fn save_json_cookies<W>(&self, mut writer: W) -> Result<(), Error>
    where
        W: std::io::Write + Send + 'static,
    {
        let cookie_store = self.cookie_store.clone();
        tokio::task::spawn_blocking(move || {
            let cookie_store = cookie_store.lock().expect("cookie store is poisoned");
            cookie_store::serde::json::save(&cookie_store, &mut writer)
                .map_err(|e| Error::CookieStore(WrapBoxError(e)))?;
            Ok(())
        })
        .await?
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

        // Initial req to login page.
        let login_page = self.scrape_webpage(LOGIN_URL).await?;
        let login_page_csrf_token = login_page
            .csrf_token
            .as_deref()
            .ok_or(Error::MissingField { name: "csrfToken" })?;
        let login_page_lu_token = login_page
            .lu_token
            .as_deref()
            .ok_or(Error::MissingField { name: "luToken" })?;

        // Get the password input page.
        // The username and password inputs are on different pages.
        let password_page_text = self
            .client
            .post("https://www.deviantart.com/_sisu/do/step2")
            .form(&[
                ("referer", LOGIN_URL),
                ("referer_type", ""),
                ("csrf_token", login_page_csrf_token),
                ("challenge", "0"),
                ("lu_token", login_page_lu_token),
                ("username", username),
                ("remember", "on"),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let password_page = tokio::task::spawn_blocking(move || {
            ScrapedWebPageInfo::from_html_str(&password_page_text)
        })
        .await??;
        let password_page_csrf_token = password_page
            .csrf_token
            .as_deref()
            .ok_or(Error::MissingField { name: "csrfToken" })?;
        let password_page_lu_token = password_page
            .lu_token
            .as_deref()
            .ok_or(Error::MissingField { name: "luToken" })?;
        let password_page_lu_token2 = password_page
            .lu_token2
            .as_deref()
            .ok_or(Error::MissingField { name: "luToken2" })?;

        // Submit password
        let signin_url = "https://www.deviantart.com/_sisu/do/signin";
        let response = self
            .client
            .post(signin_url)
            .form(&[
                ("referer", signin_url),
                ("referer_type", ""),
                ("csrf_token", password_page_csrf_token),
                ("challenge", "0"),
                ("lu_token", password_page_lu_token),
                ("lu_token2", password_page_lu_token2),
                ("username", ""),
                ("password", password),
                ("remember", "on"),
            ])
            .send()
            .await?
            .error_for_status()?;

        let text = response.text().await?;
        let scraped_webpage =
            tokio::task::spawn_blocking(move || ScrapedWebPageInfo::from_html_str(&text)).await??;
        if !scraped_webpage.is_logged_in() {
            return Err(Error::SignInFailed);
        }

        Ok(())
    }

    /// Run a GET request on the home page and check if the user is logged in
    pub async fn is_logged_in_online(&self) -> Result<bool, Error> {
        Ok(self.scrape_webpage(HOME_URL).await?.is_logged_in())
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

    /// Get the current page of deviations
    pub fn current_deviations(&self) -> Option<Result<Vec<&Deviation>, Error>> {
        let page = self.page.as_ref()?;

        let browse_page_stream = page
            .streams
            .as_ref()
            .unwrap()
            .browse_page_stream
            .as_ref()
            .unwrap();

        Some(
            browse_page_stream
                .items
                .iter()
                .filter_map(|id| {
                    // TODO: Investigate string format more.
                    id.as_u64()
                })
                .map(|id| {
                    page.get_deviation_by_id(id)
                        .ok_or(Error::MissingDeviation(id))
                })
                .collect(),
        )
    }

    /// Take the current page of deviations
    pub fn take_current_deviations(&mut self) -> Option<Result<Vec<Deviation>, Error>> {
        let mut page = self.page.take()?;

        let browse_page_stream = page
            .streams
            .as_mut()
            .unwrap()
            .browse_page_stream
            .as_mut()
            .unwrap();

        let items = std::mem::take(&mut browse_page_stream.items);
        Some(
            items
                .iter()
                .filter_map(|id| {
                    // TODO: Investigate string format more.
                    id.as_u64()
                })
                .map(|id| {
                    page.take_deviation_by_id(id)
                        .ok_or(Error::MissingDeviation(id))
                })
                .collect(),
        )
    }

    /// Get the next page, updating the internal cursor.
    pub async fn next_page(&mut self) -> Result<(), Error> {
        let page = self
            .client
            .search_raw(&self.query, self.cursor.as_deref())
            .await?;
        // Validate before storing
        match page
            .streams
            .as_ref()
            .ok_or(Error::MissingStreams)?
            .browse_page_stream
            .as_ref()
        {
            Some(browse_page_stream) => {
                self.cursor = Some(browse_page_stream.cursor.clone());
            }
            None => {
                return Err(Error::MissingBrowsePageStream);
            }
        }
        self.page = Some(page);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// The default test config path
    ///
    /// Update this if this crate is moved to a different directory relative to the workspace Cargo.toml.
    const DEFAULT_CONFIG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../", "config.json");

    #[derive(serde::Deserialize)]
    struct Config {
        username: String,
        password: String,
    }

    impl Config {
        fn from_path(path: &str) -> Option<Config> {
            let file = match std::fs::read(path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return None;
                }
                Err(error) => panic!("failed to read file: {error}"),
            };
            let config = serde_json::from_reader(file.as_slice()).expect("failed to parse config");
            Some(config)
        }

        fn from_env() -> Option<Self> {
            let username = std::env::var_os("DEVIANTART_RS_USERNAME")?
                .into_string()
                .expect("invalid `DEVIANTART_RS_USERNAME`");
            let password = std::env::var_os("DEVIANTART_RS_PASSWORD")?
                .into_string()
                .expect("invalid `DEVIANTART_RS_PASSWORD`");

            Some(Self { username, password })
        }

        fn from_any(path: &str) -> Self {
            Self::from_env()
                .or_else(|| Self::from_path(path))
                .expect("failed to load config from env or path")
        }
    }

    #[tokio::test]
    #[ignore]
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
        let config: Config = Config::from_any(DEFAULT_CONFIG_PATH);

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
    #[ignore]
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
    #[ignore]
    async fn scrape_stash_info_works() {
        let client = Client::new();
        let url = "https://sta.sh/02bhirtp3iwq";
        let stash = client
            .scrape_webpage(url)
            .await
            .expect("failed to scrape stash");
        let current_deviation_id = stash
            .get_current_deviation_id()
            .expect("missing current deviation id");
        assert!(current_deviation_id.as_u64() == Some(590293385));
    }

    #[tokio::test]
    #[ignore]
    async fn it_works() {
        let client = Client::new();
        let mut search_cursor = client.search("sun", None);
        search_cursor
            .next_page()
            .await
            .expect("failed to get next page");
        let results = search_cursor
            .current_deviations()
            .expect("missing page")
            .expect("failed to look up deviations");
        let first = &results.first().expect("no results");

        let url = first
            .get_download_url()
            .or_else(|| first.get_fullview_url())
            .expect("failed to find download url");
        let bytes = client
            .client
            .get(url)
            .send()
            .await
            .expect("failed to send")
            .error_for_status()
            .expect("bad status")
            .bytes()
            .await
            .expect("failed to buffer bytes");

        std::fs::write("test.jpg", &bytes).expect("failed to write to file");
        search_cursor
            .next_page()
            .await
            .expect("failed to get next page");
        let _results = search_cursor
            .current_deviations()
            .expect("missing page")
            .expect("failed to look up deviations");
    }
}
