use crate::util::sanitize_path;
use anyhow::Context;
use std::path::Path;

#[derive(argh::FromArgs)]
#[argh(
    subcommand,
    name = "download-stash",
    description = "download from sta.sh"
)]
pub struct Options {
    #[argh(positional, description = "the sta.sh url")]
    pub url: String,
}

pub async fn execute(client: deviantart::Client, options: Options) -> anyhow::Result<()> {
    let scraped_stash_info = client
        .scrape_stash_info(&options.url)
        .await
        .context("failed to scrape stash info")?;
    let oembed_data = client
        .get_oembed(&options.url)
        .await
        .context("failed to get oembed data")?;

    let best_film_size = scraped_stash_info
        .film
        .as_ref()
        .context("missing film")?
        .get_best_size()
        .context("missing film sizes")?;

    let best_width = best_film_size.width;
    let best_height = best_film_size.height;
    println!("Best Film Size: {best_width} x {best_height}");

    let extension = Path::new(best_film_size.src.path())
        .extension()
        .and_then(|ext| ext.to_str())
        .context("missing extension")?;

    let title = oembed_data.title;
    let deviation_id = scraped_stash_info.deviationid;
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

    nd_util::download_to_path(&client.client, best_film_size.src.as_str(), file_name)
        .await
        .context("failed to download path")?;

    Ok(())
}
