use crate::util::sanitize_path;
use anyhow::bail;
use anyhow::Context;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::io::BufWriter;

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

    println!(
        "Best Film Size: {} x {}",
        best_film_size.width, best_film_size.height
    );

    let extension = Path::new(best_film_size.src.path())
        .extension()
        .and_then(|ext| ext.to_str())
        .context("missing extension")?;

    let filename = sanitize_path(&format!(
        "{}-{}.{}",
        oembed_data.title, scraped_stash_info.deviationid, extension
    ));
    println!("Out Path: {}", filename);
    if Path::new(&filename).exists() {
        bail!("file already exists");
    }

    let mut res = client
        .client
        .get(best_film_size.src.as_str())
        .send()
        .await?
        .error_for_status()?;

    let mut file = BufWriter::new(File::create(filename).await?);
    while let Some(chunk) = res.chunk().await? {
        file.write_all(&chunk).await?;
    }

    Ok(())
}
