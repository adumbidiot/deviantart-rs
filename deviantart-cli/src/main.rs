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
        .context("Failed to start tokio runtime")?;

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
    if let Err(error) = load_cookie_jar(client).await {
        eprintln!("Failed to load cookie jar: {error:?}");
    }

    if !client
        .is_logged_in_online()
        .await
        .context("failed to check if logged in")?
    {
        match (username, password) {
            (Some(username), Some(password)) => {
                println!("Logging in...");
                client
                    .login(username, password)
                    .await
                    .context("failed to login")?;
                println!("Logged in");
                println!();

                if let Err(error) = save_cookie_jar(client)
                    .await
                    .context("Failed to save cookies")
                {
                    println!("{error:?}");
                }
            }
            (None, Some(_password)) => {
                bail!("Missing username");
            }
            (Some(_username), None) => {
                bail!("Missing password");
            }
            (None, None) => {
                bail!("Missing username and password");
            }
        }
    }

    Ok(())
}

async fn load_config_cli() -> Config {
    Config::load().await.unwrap_or_else(|error| {
        println!("failed to load config: {error:?}");
        Config::new()
    })
}

fn get_cookie_file_path() -> anyhow::Result<PathBuf> {
    let base_dirs = directories_next::BaseDirs::new().context("Failed to get base dirs")?;
    Ok(base_dirs.data_dir().join("deviantart").join("cookies.txt"))
}

async fn load_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::{fs::File, io::BufReader};

    let cookie_file = File::open(get_cookie_file_path()?).context("Failed to read cookies")?;

    let cookie_store = client.cookie_store.clone();
    tokio::task::spawn_blocking(move || {
        let new_cookie_store =
            netscape::load(&mut BufReader::new(cookie_file)).map_err(anyhow::Error::msg)?;
        let mut cookie_store = cookie_store.lock().expect("Cookie store is poisoned");
        *cookie_store = new_cookie_store;
        anyhow::Ok(())
    })
    .await??;

    Ok(())
}

async fn save_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::fs::File;

    let path = get_cookie_file_path()?;
    let mut cookie_file = File::create(path).context("Failed to create cookie file")?;
    let cookie_store = client.cookie_store.clone();
    tokio::task::spawn_blocking(move || {
        let cookie_store = cookie_store.lock().expect("Cookie store is poisoned");
        netscape::save(&cookie_store, &mut cookie_file).map_err(anyhow::Error::msg)?;
        anyhow::Ok(())
    })
    .await??;

    Ok(())
}
