use crate::load_config_cli;
use crate::try_signin_cli;
use crate::util::sanitize_path;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Context;
use deviantart::Url;
use std::fmt::Write as _;
use std::io::Write;

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
        let url = Url::parse(&options.url)?;
        let image_number = url
            .fragment()
            .and_then(|image| image.strip_prefix("image-"))
            .map(|value| value.parse::<u8>())
            .transpose()?;

        download_image_cli(
            &client,
            current_deviation,
            current_deviation_extended,
            &options,
            image_number,
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

    // We need to clone in order to pass to a tokio task.
    // TODO: Copy the text and pass that instead, parsing within the task.
    let text_content = current_deviation
        .text_content
        .as_ref()
        .context("deviation is missing text content")?
        .clone();

    let title = current_deviation.title.to_string();
    let deviation_id = current_deviation.deviation_id;
    let file_name = format!("{title}-{deviation_id}.html");
    let file_name = sanitize_path(&file_name);

    if tokio::fs::try_exists(&file_name)
        .await
        .context("failed to check if file exists")?
    {
        println!("file already exists");
        return Ok(());
    }

    let temp_path = nd_util::with_push_extension(&file_name, "part");
    let mut temp_path = tokio::task::spawn_blocking(move || {
        let temp_file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .context("failed to open temp file")?;
        let mut temp_file = std::io::BufWriter::new(temp_file);
        let temp_path = nd_util::DropRemovePath::new(temp_path);

        write_html(&title, text_content, &mut temp_file)?;

        let mut temp_file = temp_file.into_inner()?;
        temp_file.flush()?;
        temp_file.sync_all()?;

        Result::<_, anyhow::Error>::Ok(temp_path)
    })
    .await??;

    tokio::fs::rename(&temp_path, file_name)
        .await
        .context("failed to rename file")?;
    temp_path.persist();

    Ok(())
}

fn write_html<W>(
    title: &str,
    text_content: deviantart::types::deviation::TextContext,
    mut html: W,
) -> anyhow::Result<()>
where
    W: std::io::Write,
{
    let markup = text_content
        .html
        .get_markup()
        .context("deviation is missing markup")?
        .context("failed to parse markup");

    write!(&mut html, "<html>")?;
    write!(&mut html, "<head>")?;
    write!(&mut html, "<meta charset=\"UTF-8\">")?;
    write!(&mut html, "<title>{title}</title>")?;
    write!(&mut html, "<style>")?;
    let css_1 = "html { font-family: devioussans02extrabold,Helvetica Neue,Helvetica,Arial,メイリオ, meiryo,ヒラギノ角ゴ pro w3,hiragino kaku gothic pro,sans-serif; }";
    write!(&mut html, "{css_1}")?;

    let css_2 =
        "body { background-color: #06070d; margin: 0; padding-bottom: 56px; padding-top: 56px; }";
    write!(&mut html, "{css_2}")?;

    let css_3 = "h1 { color: #f2f2f2; font-weight: 400; font-size: 48px; line-height: 1.22; letter-spacing: .3px;}";
    write!(&mut html, "{css_3}")?;

    let css_4 = "p { color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px; }";
    write!(&mut html, "{css_4}")?;

    write!(&mut html, "</style>")?;
    write!(&mut html, "</head>")?;
    write!(&mut html, "<body>")?;
    write!(&mut html, "<div style=\"width:780px;margin:auto;\">")?;
    write!(&mut html, "<h1>{title}</h1>")?;

    match markup {
        Ok(markup) => {
            ensure!(markup.version == 1);
            ensure!(markup.document.kind == "doc");

            for content in markup.document.content.iter() {
                match content.kind.as_str() {
                    "paragraph" => {
                        let mut style = String::new();
                        for (key, value) in content.attrs.iter() {
                            match key.as_str() {
                                "indentation" => {
                                    let value = value.as_str().context("value is not a string")?;

                                    // IDK how to handle this.
                                    // Likely handled by adding a margin at the start.
                                    ensure!(value.is_empty());
                                }
                                "textAlign" => {
                                    let value = value.as_str().context("value is not a string")?;
                                    write!(&mut style, "text-align:{value};")?;
                                }
                                _ => bail!("unknown p attr \"{key}\""),
                            }
                        }
                        write!(&mut html, "<p style=\"{style}\">")?;

                        let content_inner_list = content
                            .content
                            .as_ref()
                            .context("p missing inner content")?;

                        for content_inner in content_inner_list {
                            match content_inner.kind.as_str() {
                                "text" => {
                                    let text =
                                        content_inner.text.as_ref().context("text missing text")?;
                                    write!(&mut html, "{text} ")?;
                                }
                                "hardBreak" => {
                                    // Deviantart seems to interpret this as 2 <br> tags.
                                    write!(&mut html, "<br><br>")?;
                                }
                                kind => bail!("unknown p inner content kind \"{kind}\""),
                            }
                        }

                        write!(&mut html, "</p>")?;
                    }
                    "da-video" => {
                        println!("Warning: video tags are currently not supported. Skipping...");
                    }
                    kind => bail!("unknown content kind \"{kind}\""),
                }
            }
        }
        Err(error) => {
            println!("Failed to parse markdown block format: {error:?}");
            println!("Interpeting as raw html...");

            let markdown = text_content
                .html
                .markup
                .as_ref()
                .context("missing markdown")?;
            write!(&mut html, "<div style=\"color: #b1b1b9; font-size: 18px; line-height: 1.5; letter-spacing: .3px;\">{markdown}</div>")?;
        }
    }

    write!(&mut html, "</div>")?;
    write!(&mut html, "</body>")?;
    write!(&mut html, "</html>")?;

    Ok(())
}

async fn download_image_cli(
    client: &deviantart::Client,
    current_deviation: &deviantart::Deviation,
    current_deviation_extended: &deviantart::DeviationExtended,
    options: &Options,
    image_number: Option<u8>,
) -> anyhow::Result<()> {
    // TODO: We should probably download every image if the image number is not specified.
    let image_number = image_number.unwrap_or(1);
    ensure!(image_number != 0);

    println!("Downloading image...");

    let extension = current_deviation
        .get_extension()
        .context("could not determine image extension")?;
    let title = current_deviation.title.as_str();
    let deviation_id = current_deviation.deviation_id;
    // TODO: We should branch here iff the deviation has multiple images.
    let file_name = if image_number == 1 {
        format!("{title}-{deviation_id}.{extension}")
    } else {
        format!("{title}-{deviation_id}-{image_number}.{extension}")
    };
    let file_name = sanitize_path(&file_name);
    println!("Out Path: {file_name}");
    if tokio::fs::try_exists(&file_name)
        .await
        .context("failed to check if file exists")?
    {
        println!("file already exists");
        return Ok(());
    }

    let url = if image_number == 1 {
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

        url
    } else {
        // -1 from handling the "1" case seperately.
        // -1 from converting this 1-based index to a 0-based index.
        let image_index = usize::from(image_number - 1 - 1);
        let additional_media = current_deviation_extended
            .additional_media
            .as_ref()
            .context("no additional media")?
            .get(image_index)
            .context("missing additional image")?;

        let mut url = None;
        if !current_deviation_extended.is_da_protected.unwrap_or(false) {
            url = additional_media.media.base_uri.clone().map(|mut url| {
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
            });
        }

        if url.is_none() && options.allow_fullview {
            url = additional_media.media.get_fullview_url();
        }
        url
    };

    let url = url.context("failed to select an image url")?;

    nd_util::download_to_path(&client.client, url.as_str(), file_name).await?;

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
    let title = current_deviation.title.as_str();
    let deviation_id = current_deviation.deviation_id;
    let file_name = format!("{title}-{deviation_id}.{extension}");
    let file_name = sanitize_path(&file_name);
    println!("Out Path: {file_name}");
    if tokio::fs::try_exists(&file_name)
        .await
        .context("failed to check if file exists")?
    {
        println!("file already exists");
        return Ok(());
    }

    let url = current_deviation
        .get_best_video_url()
        .context("missing video url")?;

    nd_util::download_to_path(&client.client, url.as_str(), file_name).await?;

    Ok(())
}
