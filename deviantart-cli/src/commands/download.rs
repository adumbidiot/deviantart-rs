use crate::load_config_cli;
use crate::try_signin_cli;
use crate::util::sanitize_path;
use anyhow::bail;
use anyhow::Context;
use std::{fmt::Write as _, path::Path};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};

#[derive(argh::FromArgs)]
#[argh(
    subcommand,
    name = "download",
    description = "download from deviantart"
)]
pub struct Options {
    #[argh(positional, description = "the deviation url")]
    pub url: String,

    #[argh(
        switch,
        description = "allow using  the fullview deviantart url, which is lower quality"
    )]
    pub allow_fullview: bool,

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

    let scraped_webpage_info = client
        .scrape_webpage(&options.url)
        .await
        .context("failed to scrape webpage")?;
    let current_deviation = scraped_webpage_info
        .get_current_deviation()
        .context("failed to get current deviation")?;
    let current_deviation_extended = scraped_webpage_info
        .get_current_deviation_extended()
        .context("failed to get current deviation extended")?;

    println!("Title: {}", current_deviation.title);
    println!("ID: {}", current_deviation.deviation_id);
    println!("Kind: {}", current_deviation.kind);
    println!("Url: {}", current_deviation.url);
    println!("Is Downloadable: {}", current_deviation.is_downloadable);
    println!(
        "Description: {}",
        current_deviation_extended
            .description
            .as_deref()
            .unwrap_or("(none)")
    );
    println!();

    if current_deviation.is_literature() {
        download_literature_cli(current_deviation).await?;
    } else if current_deviation.is_image() {
        download_image_cli(
            &client,
            current_deviation,
            current_deviation_extended,
            &options,
        )
        .await?;
    } else if current_deviation.is_film() {
        download_film_cli(&client, current_deviation).await?;
    } else {
        bail!("unknown deviation type: {}", current_deviation.kind);
    }

    Ok(())
}

async fn download_literature_cli(current_deviation: &deviantart::Deviation) -> anyhow::Result<()> {
    println!("Generating html page...");

    let text_content = current_deviation
        .text_content
        .as_ref()
        .context("deviation is missing text content")?;
    let markup = text_content
        .html
        .get_markup()
        .context("deviation is missing markup")?
        .context("failed to parse markup");

    let filename = sanitize_path(&format!(
        "{}-{}.html",
        current_deviation.title, current_deviation.deviation_id
    ));

    if Path::new(&filename).exists() {
        bail!("file already exists");
    }

    let mut html = String::with_capacity(1_000_000); // 1 MB

    html.push_str("<html>");
    html.push_str("<head>");
    html.push_str("<meta charset=\"UTF-8\">");
    write!(&mut html, "<title>{}</title>", &current_deviation.title)?;
    html.push_str("<style>");
    html.push_str("html { font-family: devioussans02extrabold,Helvetica Neue,Helvetica,Arial,メイリオ, meiryo,ヒラギノ角ゴ pro w3,hiragino kaku gothic pro,sans-serif; }");
    html.push_str(
        "body { background-color: #06070d; margin: 0; padding-bottom: 56px; padding-top: 56px; }",
    );
    html.push_str("h1 { color: #f2f2f2; font-weight: 400; font-size: 48px; line-height: 1.22; letter-spacing: .3px;}");
    html.push_str(
        "span { color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px; }",
    );
    html.push_str("</style>");
    html.push_str("</head>");

    html.push_str("<body>");

    html.push_str("<div style=\"width:780px;margin:auto;\">");
    write!(&mut html, "<h1>{}</h1>", &current_deviation.title)?;

    match markup {
        Ok(markup) => {
            for block in markup.blocks.iter() {
                write!(&mut html, "<div id = \"{}\">", block.key)?;

                html.push_str("<span>");
                if block.text.is_empty() {
                    html.push_str("<br>");
                } else {
                    html.push_str(&block.text);
                }
                html.push_str("</span>");

                html.push_str("</div>");
            }
        }
        Err(e) => {
            println!("Failed to parse markdown block format: {:?}", e);
            println!("Interpeting as raw html...");

            write!(&mut html, "<div style=\"color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px;\">{}</div>", text_content.html.markup.as_ref().context("missing markdown")?)?;
        }
    }

    html.push_str("</div>");
    html.push_str("</body>");
    html.push_str("</html>");

    tokio::fs::write(filename, html).await?;

    Ok(())
}

async fn download_image_cli(
    client: &deviantart::Client,
    current_deviation: &deviantart::Deviation,
    current_deviation_extended: &deviantart::DeviationExtended,
    options: &Options,
) -> anyhow::Result<()> {
    println!("Downloading image...");

    let extension = current_deviation
        .get_extension()
        .context("could not determine image extension")?;
    let filename = sanitize_path(&format!(
        "{}-{}.{}",
        current_deviation.title, current_deviation.deviation_id, extension
    ));
    println!("Out Path: {}", filename);
    if Path::new(&filename).exists() {
        bail!("file already exists");
    }

    let mut url = current_deviation_extended
        .download
        .as_ref()
        .map(|download| download.url.clone())
        .or_else(|| current_deviation.get_image_download_url());

    // This is not default as a "fullview" can be thought of as a "preview".
    // It's not an actual download, but helps when downloads are disabled.
    if url.is_none() && options.allow_fullview {
        url = current_deviation.get_fullview_url();
    }

    let url = url.context("failed to select an image url")?;

    let bytes = client
        .client
        .get(url.as_str())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    tokio::fs::write(filename, bytes).await?;

    Ok(())
}

async fn download_film_cli(
    client: &deviantart::Client,
    current_deviation: &deviantart::Deviation,
) -> anyhow::Result<()> {
    println!("Downloading video...");

    let extension = current_deviation
        .get_extension()
        .context("could not determine video extension")?;
    let filename = sanitize_path(&format!(
        "{}-{}.{}",
        current_deviation.title, current_deviation.deviation_id, extension
    ));
    println!("Out Path: {}", filename);
    if Path::new(&filename).exists() {
        anyhow::bail!("file already exists");
    }

    let url = current_deviation
        .get_best_video_url()
        .context("missing video url")?;

    let mut res = client
        .client
        .get(url.as_str())
        .send()
        .await?
        .error_for_status()?;

    let mut file = BufWriter::new(File::create(filename).await?);
    while let Some(chunk) = res.chunk().await? {
        file.write_all(&chunk).await?;
    }

    Ok(())
}
