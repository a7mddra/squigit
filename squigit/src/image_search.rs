use anyhow::{Context, Result};
use reqwest::blocking::{multipart, Client};
use serde::Deserialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

use crate::{auth, notifications, settings, storage::AppData};

pub enum Target {
    Lens,
    Ai(String),
}

pub fn submit(
    data: AppData,
    image: PathBuf,
    target: Target,
    settings: rune::AppHandle<settings::App>,
) {
    std::thread::spawn(move || {
        let result = (|| {
            let Some(key) = auth::load(&data)? else {
                notifications::image_search_error(
                    "Add your ImgBB API key to use image search.".into(),
                    true,
                    settings.clone(),
                );
                return Ok(());
            };
            let hosted = upload(&image, &key)?;
            let url = google_url(&hosted, target)?;
            opener::open_browser(url.as_str())
                .context("Could not open image search in your browser")?;
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = result {
            notifications::image_search_error(format!("{error:#}"), false, settings);
        }
    });
}

#[derive(Deserialize)]
struct UploadResponse {
    success: bool,
    data: Option<UploadData>,
    error: Option<UploadError>,
}

#[derive(Deserialize)]
struct UploadData {
    url: String,
}

#[derive(Deserialize)]
struct UploadError {
    message: Option<String>,
}

fn upload(image: &Path, key: &auth::SecretString) -> Result<Url> {
    let size = std::fs::metadata(image)
        .context("Could not read the captured image")?
        .len();
    anyhow::ensure!(
        size > 0 && size <= 32 * 1024 * 1024,
        "ImgBB requires an image between 1 byte and 32 MB"
    );
    let image = multipart::Part::file(image)
        .context("Could not open the captured image")?
        .mime_str("image/png")?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client
        .post("https://api.imgbb.com/1/upload")
        .multipart(
            multipart::Form::new()
                .text("key", key.expose().to_owned())
                .part("image", image),
        )
        .send()
        .map_err(|error| {
            anyhow::anyhow!(
                "Could not upload the image to ImgBB: {}",
                error.without_url()
            )
        })?;
    let status = response.status();
    let mut body = Vec::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_end(&mut body)
        .context("Could not read the ImgBB response")?;
    anyhow::ensure!(
        body.len() <= 1024 * 1024,
        "ImgBB returned an oversized response"
    );
    let parsed = serde_json::from_slice::<UploadResponse>(&body);
    if !status.is_success() {
        anyhow::bail!(
            "ImgBB rejected the upload ({status}). Check your API key in Image hosting settings."
        );
    }
    let response = parsed.context("ImgBB returned an invalid response")?;
    anyhow::ensure!(
        response.success,
        "{}",
        response
            .error
            .and_then(|error| error.message)
            .unwrap_or_else(|| "ImgBB could not upload the image".into())
            .replace(key.expose(), "[REDACTED]")
    );
    let url = Url::parse(
        &response
            .data
            .context("ImgBB did not return an image URL")?
            .url,
    )
    .context("ImgBB returned an invalid image URL")?;
    anyhow::ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "ImgBB returned an unsafe image URL"
    );
    Ok(url)
}

fn google_url(image: &Url, target: Target) -> Result<Url> {
    let mut url = Url::parse("https://lens.google.com/uploadbyurl")?;
    url.query_pairs_mut()
        .append_pair("url", image.as_str())
        .append_pair("ep", "subb")
        .append_pair("re", "df")
        .append_pair("s", "4");
    if let Target::Ai(prompt) = target {
        anyhow::ensure!(!prompt.trim().is_empty(), "Enter a prompt for Ask Google");
        url.query_pairs_mut()
            .append_pair("udm", "50")
            .append_pair("q", prompt.trim());
    }
    Ok(url)
}
