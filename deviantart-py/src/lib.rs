use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
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

    pub fn get_deviation(&self, url: String) -> PyResult<Deviation> {
        let tokio_rt = TOKIO_RT
            .as_ref()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let scraped_webpage_info = tokio_rt
            .block_on(async { self.client.scrape_webpage(url.as_str()).await })
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;

        let current_deviation = scraped_webpage_info
            .get_current_deviation()
            .ok_or_else(|| PyRuntimeError::new_err("failed to get current deviation"))?;

        let current_deviation_extended = scraped_webpage_info
            .get_current_deviation_extended()
            .ok_or_else(|| PyRuntimeError::new_err("failed to get current deviation extended"))?;

        Ok(Deviation {
            id: current_deviation.deviation_id,
            title: current_deviation.title.clone(),
            description: current_deviation_extended.description.clone(),
            kind: current_deviation.kind.clone(),
            download_url: current_deviation.get_download_url().map(String::from),
        })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[pyclass]
pub struct Deviation {
    #[pyo3(get)]
    pub id: u64,

    #[pyo3(get)]
    pub title: String,

    #[pyo3(get)]
    pub description: Option<String>,

    #[pyo3(get, name = "type")]
    pub kind: String,

    #[pyo3(get)]
    pub download_url: Option<String>,
}

#[pymethods]
impl Deviation {
    pub fn __repr__(&self) -> String {
        let id = &self.id;
        let kind = &self.kind;
        let title = &self.title;
        let description = &self.description;
        let download_url = &self.download_url;

        format!("Deviation(id={id}, type={kind:?}, title={title:?}, description={description:?}, download_url={download_url:?})")
    }
}

/// A Python module for accessing deviantart.
#[pymodule]
fn deviantart_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    Ok(())
}
