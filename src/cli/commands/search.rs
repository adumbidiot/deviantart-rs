use crate::{load_config_cli, try_signin_cli};
use anyhow::Context;

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "search")]
#[argh(description = "search on deviantart")]
pub struct Options {
    #[argh(positional, description = "the query string")]
    pub query: String,

    #[argh(option, short = 'c', long = "cursor", description = "the page cursor")]
    pub cursor: Option<String>,

    #[argh(switch, long = "no-login", description = "do not try to log in")]
    pub no_login: bool,
}

pub async fn execute(client: deviantart::Client, options: Options) -> anyhow::Result<()> {
    let config = load_config_cli().await;

    if !options.no_login {
        try_signin_cli(
            &client,
            config.username.as_deref(),
            config.password.as_deref(),
        )
        .await?;
    }

    let mut search_cursor = client.search(&options.query, options.cursor.as_deref());
    let results = search_cursor
        .next_page()
        .await
        .with_context(|| format!("failed to search for '{}'", &options.query))?;

    if results.is_empty() {
        println!("no results for '{}'", &options.query);
    } else {
        println!("Results");
        for (i, deviation) in results.iter().enumerate() {
            println!("{}) {}", i + 1, deviation.title);
            println!("Id: {}", deviation.deviation_id);
            println!("Kind: {}", deviation.kind);
            println!("Url: {}", deviation.url);
            println!("Is downloadable: {}", deviation.is_downloadable);
            println!();
        }
    }

    Ok(())
}
