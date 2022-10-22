mod commands;
mod config;

use self::config::Config;
use anyhow::bail;
use anyhow::Context;
use std::path::PathBuf;

#[derive(Debug)]
struct WrapBoxError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl std::fmt::Display for WrapBoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for WrapBoxError {}

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
    DownloadStash(self::commands::download_stash::Options),
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
    eprintln!("Done.");

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
        SubCommand::DownloadStash(options) => {
            self::commands::download_stash::execute(client, options).await?;
        }
    }

    Ok(())
}

async fn try_signin_cli(
    client: &deviantart::Client,
    username: Option<&str>,
    password: Option<&str>,
) -> anyhow::Result<()> {
    if let Err(e) = load_cookie_jar(client) {
        eprintln!("Failed to load cookie jar: {:?}", e);
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
                    .sign_in(username, password)
                    .await
                    .context("failed to login")?;
                println!("logged in");
                println!();

                if let Err(e) = save_cookie_jar(client).context("failed to save cookies") {
                    println!("{:?}", e);
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
        println!("failed to load config: {:?}", e);
        Config::new()
    })
}

fn get_cookie_file_path() -> anyhow::Result<PathBuf> {
    let base_dirs = directories_next::BaseDirs::new().context("failed to get base dirs")?;
    Ok(base_dirs.data_dir().join("deviantart/cookies.json"))
}

fn load_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::{fs::File, io::BufReader};

    let cookie_file = File::open(get_cookie_file_path()?).context("failed to read cookies")?;
    let mut cookie_store = client
        .cookie_store
        .lock()
        .expect("cookie store is poisoned");
    let new_cookie_store =
        reqwest_cookie_store::CookieStore::load_json(BufReader::new(cookie_file))
            .map_err(WrapBoxError)?;
    *cookie_store = new_cookie_store;

    Ok(())
}

fn save_cookie_jar(client: &deviantart::Client) -> anyhow::Result<()> {
    use std::fs::File;

    let mut cookie_file =
        File::create(get_cookie_file_path()?).context("failed to create cookie file")?;
    let cookie_store = client
        .cookie_store
        .lock()
        .expect("cookie store is poisoned");
    cookie_store
        .save_json(&mut cookie_file)
        .map_err(WrapBoxError)?;

    Ok(())
}

fn escape_filename(path: &str) -> String {
    path.chars()
        .map(|c| {
            if [':', '?', '/', '|', '*'].contains(&c) {
                '-'
            } else {
                c
            }
        })
        .collect()
}
