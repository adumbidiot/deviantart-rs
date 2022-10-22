use anyhow::Context;
use directories_next::BaseDirs;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            username: None,
            password: None,
        }
    }

    pub fn get_config_dir() -> anyhow::Result<PathBuf> {
        let base_dirs = BaseDirs::new().context("failed to get base dirs")?;
        Ok(base_dirs.config_dir().join("deviantart"))
    }

    pub fn get_config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::get_config_dir()?.join("config.toml"))
    }

    pub async fn create_config_path() -> anyhow::Result<()> {
        tokio::fs::create_dir_all(Self::get_config_dir()?)
            .await
            .context("failed to create config dir")?;

        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        Self::create_config_path().await?;

        let config_path = Self::get_config_path()?;
        let mut new_config = Self::load().await.unwrap_or_else(|_| Self::new());

        if let Some(username) = self.username.clone() {
            new_config.username = Some(username);
        }

        if let Some(password) = self.password.clone() {
            new_config.password = Some(password);
        }

        let toml_str = toml::to_string_pretty(&new_config).context("failed to serialize config")?;

        tokio::fs::write(config_path, toml_str)
            .await
            .context("failed to write config")?;

        Ok(())
    }

    pub async fn load() -> anyhow::Result<Self> {
        Self::create_config_path().await?;

        let config_path = Self::get_config_path()?;

        let config_str = tokio::fs::read_to_string(config_path)
            .await
            .context("failed to read config file")?;
        toml::from_str(&config_str).context("failed to parse config")
    }
}
