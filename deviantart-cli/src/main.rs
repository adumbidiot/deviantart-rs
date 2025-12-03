mod commands;
mod config;
pub mod util;

use self::config::Config;
use anyhow::Context;
use anyhow::bail;
use std::path::PathBuf;

#[derive(argh::FromArgs)]
#[argh(description = "a tool to interact with deviantart")]
struct Options {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    Login(self::commands::login::Options),
    Search(self::commands::search::Options),
    Download(self::commands::download::Options),
}

fn main() -> anyhow::Result<()> {
    let options: Options = argh::from_env();
    real_main(options)
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start tokio runtime")?;

    tokio_rt.block_on(async_main(options))?;

    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = deviantart::Client::new();

    match options.subcommand {
        SubCommand::Login(options) => {
            self::commands::login::execute(client, options).await?;
        }
        SubCommand::Search(options) => {
            self::commands::search::execute(client, options).await?;
        }
        SubCommand::Download(options) => {
            self::commands::download::execute(client, options).await?;
        }
    }

    Ok(())
}

async fn try_signin_cli(
    client: &deviantart::Client,
    username: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<()> {
    if let Err(e) = load_cookie_jar(client).await {
        eprintln!("failed to load cookie jar: {e:?}");
    }

    if !client
        .is_logged_in_online()
        .await
        .context("failed to check if logged in")?
    {
        match (username, password) {
            (Some(username), Some(password)) => {
                println!("logging in...");
                client
                    .login(username, password)
                    .await
                    .context("failed to login")?;
                println!("logged in");
                println!();

                if let Err(e) = save_cookie_jar(client)
                    .await
                    .context("failed to save cookies")
                {
                    println!("{e:?}");
                }
            }
            (None, Some(_password)) => {
                bail!("missing username");
            }
            (Some(_username), None) => {
                bail!("missing password");
            }
            (None, None) => {
                bail!("missing username and password");
            }
        }
    }

    Ok(())
}

async fn load_config_cli() -> Config {
    Config::load().await.unwrap_or_else(|e| {
        println!("failed to load config: {e:?}");
        Config::new()
    })
}

fn get_cookie_file_path() -> anyhow::Result<PathBuf> {
    let base_dirs = directories_next::BaseDirs::new().context("failed to get base dirs")?;
    Ok(base_dirs.data_dir().join("deviantart/cookies.json"))
}

async fn load_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::{fs::File, io::BufReader};

    let cookie_file = File::open(get_cookie_file_path()?).context("failed to read cookies")?;
    client
        .load_json_cookies(BufReader::new(cookie_file))
        .await?;
    Ok(())
}

async fn save_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::fs::File;

    let cookie_file =
        File::create(get_cookie_file_path()?).context("failed to create cookie file")?;
    client.save_json_cookies(cookie_file).await?;

    Ok(())
}
