use crate::{config::Config, get_cookie_file_path};
use anyhow::Context;

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "login")]
#[argh(description = "login on deviantart and save the credentials to a file")]
pub struct Options {
    #[argh(option, description = "your username", short = 'u', long = "username")]
    pub username: String,

    #[argh(option, description = "your password", short = 'p', long = "password")]
    pub password: String,
}

pub async fn execute(_client: deviantart::Client, options: Options) -> anyhow::Result<()> {
    // TODO: Consider verifying login online + saving new cookies

    let mut config = Config::new();
    config.username = Some(options.username);
    config.password = Some(options.password);
    config.save().await.context("failed to save config")?;
    if let Err(e) = tokio::fs::remove_file(get_cookie_file_path()?).await {
        eprintln!("Failed to delete old cookie file: {}", e);
    }

    Ok(())
}
