use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyString;
use pyo3::types::PyStringMethods;
use std::sync::LazyLock;

static TOKIO_RT: LazyLock<std::io::Result<tokio::runtime::Runtime>> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
});

#[pyclass]
pub struct Client {
    client: deviantart::Client,
}

#[pymethods]
impl Client {
    #[new]
    pub fn new() -> Self {
        Self {
            client: deviantart::Client::new(),
        }
    }

    /// Get metadata for a deviation.
    pub fn get_deviation(&self, source: Bound<'_, PyAny>) -> PyResult<Deviation> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let source = if let Ok(url) = source.downcast::<PyString>() {
            url.to_cow()?.into_owned()
        } else if let Ok(id) = source.extract::<u64>() {
            format!("https://www.deviantart.com/view/{id}")
        } else {
            return Err(PyValueError::new_err(
                "source must be a deviation id or a url",
            ));
        };

        let scraped_webpage_info = tokio_rt
            .block_on(async { self.client.scrape_webpage(source.as_str()).await })
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let current_deviation = scraped_webpage_info
            .get_current_deviation()
            .ok_or_else(|| PyRuntimeError::new_err("failed to get current deviation"))?;

        let current_deviation_extended = scraped_webpage_info
            .get_current_deviation_extended()
            .ok_or_else(|| PyRuntimeError::new_err("failed to get current deviation extended"))?;

        let download_url = current_deviation_extended
            .download
            .as_ref()
            .map(|download| download.url.clone())
            .or_else(|| current_deviation.get_download_url())
            .map(String::from);

        let additional_media_download_urls = current_deviation_extended
            .additional_media
            .as_ref()
            .map(|additional_media| {
                additional_media
                    .iter()
                    .map(|additional_media| {
                        Some(additional_media.media.base_uri.as_ref()?.to_string())
                    })
                    .collect()
            });

        Ok(Deviation {
            id: current_deviation.deviation_id,
            title: current_deviation.title.clone(),
            description: current_deviation_extended.description.clone(),
            kind: current_deviation.kind.clone(),
            download_url,
            additional_media_download_urls,
        })
    }

    /// Download a deviation.
    pub fn download_deviation<'p>(
        &self,
        deviation: &Deviation,
        py: Python<'p>,
    ) -> PyResult<Bound<'p, PyBytes>> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let download_url = deviation
            .download_url
            .as_ref()
            .ok_or(PyValueError::new_err("deviation is missing a download url"))?;

        let bytes = tokio_rt
            .block_on(async {
                self.client
                    .client
                    .get(download_url)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await
            })
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        Ok(PyBytes::new(py, &bytes))
    }

    /// Check if this client is logged in.
    pub fn is_logged_in(&self) -> PyResult<bool> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        tokio_rt
            .block_on(self.client.is_logged_in_online())
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    /// Load cookies.
    pub fn load_cookies_json(&self, cookie_json_string: String) -> PyResult<()> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        tokio_rt
            .block_on(
                self.client
                    .load_json_cookies(std::io::Cursor::new(cookie_json_string)),
            )
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    /// Dump cookies.
    pub fn dump_cookies_json(&self) -> PyResult<String> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let buffer = Vec::new();
        let buffer = tokio_rt
            .block_on(self.client.save_json_cookies(buffer))
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        Ok(String::from_utf8(buffer)?)
    }

    /// Log in with this client.
    pub fn login(&self, username: &str, password: &str) -> PyResult<()> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        tokio_rt
            .block_on(self.client.sign_in(username, password))
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    /// Get the folder given by the url.
    pub fn get_folder(&self, url: &str) -> PyResult<Folder> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let scraped_webpage_info = tokio_rt
            .block_on(async { self.client.scrape_webpage(url).await })
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let id = scraped_webpage_info
            .get_current_folder_id()
            .ok_or_else(|| PyRuntimeError::new_err("missing folder id"))?;

        let stream = scraped_webpage_info
            .get_folder_deviations_stream(id)
            .ok_or_else(|| PyRuntimeError::new_err("missing folder deviation stream"))?;

        let deviation_ids = stream.items.clone();

        Ok(Folder {
            id,
            deviation_ids,
            has_more: stream.has_more,
        })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Deviation {
    #[pyo3(set, get)]
    pub id: u64,

    #[pyo3(set, get)]
    pub title: String,

    #[pyo3(set, get)]
    pub description: Option<String>,

    #[pyo3(set, get, name = "type")]
    pub kind: String,

    #[pyo3(set, get)]
    pub download_url: Option<String>,

    #[pyo3(get)]
    pub additional_media_download_urls: Option<Vec<Option<String>>>,
}

#[pymethods]
impl Deviation {
    /// Dump this to a json string.
    ///
    /// Be very careful about using this for caching.
    /// The embedded download urls can and will expire.
    #[pyo3(signature=(pretty=false))]
    pub fn to_json(&self, pretty: bool) -> PyResult<String> {
        let result = if pretty {
            serde_json::to_string_pretty(&self)
        } else {
            serde_json::to_string(&self)
        };

        result.map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    /// Parse this from a json string.
    ///
    /// Be very careful about using this for caching.
    /// The embedded download urls can and will expire.
    #[staticmethod]
    pub fn from_json(value: &str) -> PyResult<Self> {
        serde_json::from_str(value).map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    pub fn __repr__(&self) -> String {
        let id = &self.id;
        let kind = &self.kind;
        let title = &self.title;
        let description = &self.description;
        let download_url = &self.download_url;
        let additional_media_download_urls = &self.additional_media_download_urls;

        format!("Deviation(id={id}, type={kind:?}, title={title:?}, description={description:?}, download_url={download_url:?}, additional_media_download_urls={additional_media_download_urls:?})")
    }
}

#[pyclass]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Folder {
    #[pyo3(get, set)]
    pub id: u64,

    #[pyo3(get, set)]
    pub deviation_ids: Vec<u64>,

    #[pyo3(get, set)]
    pub has_more: bool,
}

#[pymethods]
impl Folder {
    /// Dump this to a json string.
    #[pyo3(signature=(pretty=false))]
    pub fn to_json(&self, pretty: bool) -> PyResult<String> {
        let result = if pretty {
            serde_json::to_string_pretty(&self)
        } else {
            serde_json::to_string(&self)
        };

        result.map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    /// Parse this from a json string.
    #[staticmethod]
    pub fn from_json(value: &str) -> PyResult<Self> {
        serde_json::from_str(value).map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    pub fn __repr__(&self) -> String {
        let id = &self.id;
        let deviation_ids = &self.deviation_ids;
        let has_more = if self.has_more { "True" } else { "False" };

        format!("Folder(id={id}, deviation_ids={deviation_ids:?}, has_more={has_more})")
    }
}

/// A Python module for accessing deviantart.
#[pymodule]
fn deviantart_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_class::<Deviation>()?;
    m.add_class::<Folder>()?;
    Ok(())
}
