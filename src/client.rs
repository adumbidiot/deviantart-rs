use crate::Error;
use crate::ScrapedWebPageInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest_cookie_store::CookieStoreMutex;
use std::sync::Arc;

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
        static REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"window\.__INITIAL_STATE__ = JSON\.parse\("(.*)"\);"#)
                .expect("invalid `scrape_deviation` regex")
        });

        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let scraped_webpage = tokio::task::spawn_blocking(move || {
            let capture = REGEX
                .captures(&text)
                .and_then(|captures| captures.get(1))
                .ok_or(Error::MissingInitialState)?;
            // TODO: Escape properly
            let capture = capture
                .as_str()
                .replace("\\\"", "\"")
                .replace("\\'", "'")
                .replace("\\\\", "\\");
            let scraped_webpage: ScrapedWebPageInfo = serde_json::from_str(&capture)?;

            Result::<_, Error>::Ok(scraped_webpage)
        })
        .await??;

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

        // TODO: Verify login
        let _text = res.text().await?;

        Ok(())
    }

    /// Run a GET request on the home page and check if the user is logged in
    pub async fn is_logged_in_online(&self) -> Result<bool, Error> {
        Ok(self
            .scrape_webpage("https://www.deviantart.com/")
            .await?
            .public_session
            .is_logged_in)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
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
}
