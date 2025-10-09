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

fn get_url_file_name(url: &str) -> PyResult<String> {
    let url =
        deviantart::Url::parse(url).map_err(|error| PyRuntimeError::new_err(error.to_string()))?;
    let file_name = url
        .path_segments()
        .ok_or_else(|| PyRuntimeError::new_err("missing path"))?
        .next_back()
        .ok_or_else(|| PyRuntimeError::new_err("missing file name"))?;

    Ok(file_name.to_string())
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

    #[pyo3(set, get)]
    pub fullview_url: Option<String>,

    #[pyo3(get)]
    pub additional_media_download_urls: Option<Vec<Option<String>>>,

    #[pyo3(get)]
    pub additional_media_fullview_urls: Option<Vec<Option<String>>>,
}

#[pymethods]
impl Deviation {
    /// Get the name of the file.
    #[pyo3(signature=(r#type="download"))]
    pub fn get_file_name(&self, r#type: &str) -> PyResult<Option<String>> {
        match r#type {
            "download" => self
                .download_url
                .as_deref()
                .map(get_url_file_name)
                .transpose(),
            "fullview" => self
                .fullview_url
                .as_deref()
                .map(get_url_file_name)
                .transpose(),
            _ => Err(PyValueError::new_err(
                "type must be \"download\" or \"fullview\"",
            )),
        }
    }

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
        let additional_media_download_urls = &self.additional_media_download_urls;

        format!("Deviation(id={id}, type={kind:?}, title={title:?}, description={description:?}, additional_media_download_urls={additional_media_download_urls:?})")
    }
}

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

        let fullview_url = current_deviation.get_fullview_url().map(String::from);

        let additional_media_download_urls = current_deviation_extended
            .additional_media
            .as_ref()
            .map(|additional_media| {
                additional_media
                    .iter()
                    .map(|additional_media| {
                        if current_deviation_extended.is_da_protected.unwrap_or(false) {
                            return None;
                        }

                        additional_media
                            .media
                            .base_uri
                            .clone()
                            .map(|mut url| {
                                // Some images require a token, some don't.
                                // I don't know what causes the token to be required.
                                // Regardless, always sending a token when possible doesn't seem to cause issues.
                                match additional_media.media.token.first().as_ref() {
                                    Some(token) => {
                                        url.query_pairs_mut().append_pair("token", token);
                                        url
                                    }
                                    None => url,
                                }
                            })
                            .map(String::from)
                    })
                    .collect()
            });
        let additional_media_fullview_urls = current_deviation_extended
            .additional_media
            .as_ref()
            .map(|additional_media| {
                additional_media
                    .iter()
                    .map(|additional_media| {
                        additional_media.media.get_fullview_url().map(String::from)
                    })
                    .collect()
            });

        Ok(Deviation {
            id: current_deviation.deviation_id,
            title: current_deviation.title.clone(),
            description: current_deviation_extended.description.clone(),
            kind: current_deviation.kind.clone(),
            download_url,
            fullview_url,
            additional_media_download_urls,
            additional_media_fullview_urls,
        })
    }

    /// Download a deviation.
    #[pyo3(signature = (deviation, use_fullview=false))]
    pub fn download_deviation<'p>(
        &self,
        deviation: &Deviation,
        use_fullview: bool,
        py: Python<'p>,
    ) -> PyResult<Bound<'p, PyBytes>> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let url = if use_fullview {
            deviation
                .fullview_url
                .as_ref()
                .ok_or(PyValueError::new_err("deviation is missing a fullview url"))?
        } else {
            deviation
                .download_url
                .as_ref()
                .ok_or(PyValueError::new_err("deviation is missing a download url"))?
        };

        let bytes = tokio_rt
            .block_on(async {
                self.client
                    .client
                    .get(url)
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
    pub fn load_cookies(&self, cookie_json_string: String) -> PyResult<()> {
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
    pub fn dump_cookies(&self) -> PyResult<String> {
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
            .block_on(self.client.login(username, password))
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

        let mut deviation_ids = stream.items.clone();

        let folder_entity = scraped_webpage_info
            .get_gallery_folder_entity(id)
            .ok_or_else(|| PyRuntimeError::new_err("missing gallery folder entity"))?;

        let user_entity = scraped_webpage_info
            .get_user_entity(folder_entity.owner)
            .ok_or_else(|| PyRuntimeError::new_err("missing user entity"))?;

        let owner_name = user_entity.username.clone();

        if stream.has_more {
            tokio_rt
                .block_on(async {
                    let mut has_more = true;
                    while has_more {
                        let offset = u64::try_from(deviation_ids.len()).unwrap();
                        let response = self
                            .client
                            .list_folder_contents(
                                &owner_name,
                                id,
                                offset,
                                &scraped_webpage_info.config.csrf_token,
                            )
                            .await?;
                        deviation_ids.extend(
                            response
                                .results
                                .iter()
                                .map(|deviation| deviation.deviation_id),
                        );
                        has_more = response.has_more;
                    }

                    Result::<_, deviantart::Error>::Ok(())
                })
                .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;
        }

        Ok(Folder {
            id,
            name: folder_entity.name.clone(),
            owner_name,
            deviation_ids,
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
pub struct Folder {
    #[pyo3(get, set)]
    pub id: u64,

    #[pyo3(set, get)]
    pub name: String,

    #[pyo3(set, get)]
    pub owner_name: String,

    #[pyo3(get, set)]
    pub deviation_ids: Vec<u64>,
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
        let name = &self.name;
        let owner_name = &self.owner_name;
        let deviation_ids = &self.deviation_ids;

        format!(
            "Folder(id={id}, name={name}, owner_name={owner_name}, deviation_ids={deviation_ids:?})"
        )
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
