use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use reqwest::blocking::{multipart, Client, Response};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use std::{
    io::Read,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

use crate::{
    auth::SecretString,
    storage::{AppData, HostingConfig},
};

pub const LINK_LIFETIME: u64 = 600;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Imgbb,
    Imagekit,
    Chevereto,
}

impl Provider {
    pub const ALL: [Self; 3] = [Self::Imgbb, Self::Imagekit, Self::Chevereto];

    pub fn index(self) -> usize {
        match self {
            Self::Imgbb => 0,
            Self::Imagekit => 1,
            Self::Chevereto => 2,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Imgbb => "imgbb",
            Self::Imagekit => "imagekit",
            Self::Chevereto => "chevereto",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Imgbb => "ImgBB",
            Self::Imagekit => "ImageKit",
            Self::Chevereto => "Custom / Chevereto",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Self::Imgbb => "Free links with 10-minute auto-deletion.\nAnyone with the link can view the image.",
            Self::Imagekit => "Private uploads with 10-minute signed links.\nFree plan available; files remain saved.",
            Self::Chevereto => "Upload to your own Chevereto instance.\nYou control hosting and retention.",
        }
    }

    pub fn dashboard(self) -> &'static str {
        match self {
            Self::Imgbb => "https://api.imgbb.com/",
            Self::Imagekit => "https://imagekit.io/dashboard/developer/api-keys",
            Self::Chevereto => "https://v4-docs.chevereto.com/api/1/file-upload.html",
        }
    }

    pub fn field_key(self) -> &'static str {
        match self {
            Self::Imgbb => "imgbb-key",
            Self::Imagekit => "imagekit-key",
            Self::Chevereto => "chevereto-key",
        }
    }

    pub fn valid_key(self, value: &str) -> bool {
        let value = value.trim();
        match self {
            Self::Imgbb => value.len() == 32 && value.bytes().all(|b| b.is_ascii_alphanumeric()),
            _ => {
                !value.is_empty()
                    && value.len() <= 4096
                    && value.bytes().all(|b| b.is_ascii_graphic())
            }
        }
    }
}

pub fn custom_endpoint(base: &str) -> Result<Url> {
    let mut base = image_url(base.trim()).context("Enter a valid HTTPS Chevereto instance URL")?;
    anyhow::ensure!(
        base.query().is_none(),
        "The instance URL must not contain query parameters"
    );
    base.set_path(&format!("{}/", base.path().trim_end_matches('/')));
    base.join("api/1/upload")
        .context("Could not construct the Chevereto upload URL")
}

pub fn configured(data: &AppData, config: &HostingConfig) -> Result<bool> {
    Ok(crate::auth::configured(data, config.provider)?
        && (config.provider != Provider::Chevereto
            || custom_endpoint(&config.chevereto_url).is_ok()))
}

pub fn upload(image: &Path, key: &SecretString, config: &HostingConfig) -> Result<Url> {
    let provider = config.provider;
    let size = std::fs::metadata(image)
        .context("Could not read the captured image")?
        .len();
    let limit = if provider == Provider::Imagekit {
        25
    } else {
        32
    } * 1024
        * 1024;
    anyhow::ensure!(
        size > 0 && size <= limit,
        "{} requires an image between 1 byte and {} MB",
        provider.name(),
        limit / 1024 / 1024
    );
    let file = multipart::Part::file(image)
        .context("Could not open the captured image")?
        .file_name(format!("squigit-{:032x}.png", rand::random::<u128>()))
        .mime_str("image/png")?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let request = match provider {
        Provider::Imgbb => client.post("https://api.imgbb.com/1/upload").multipart(
            multipart::Form::new()
                .text("key", key.expose().to_owned())
                .text("expiration", LINK_LIFETIME.to_string())
                .part("image", file),
        ),
        Provider::Imagekit => client
            .post("https://upload.imagekit.io/api/v1/files/upload")
            .basic_auth(key.expose(), Some(""))
            .multipart(
                multipart::Form::new()
                    .part("file", file)
                    .text(
                        "fileName",
                        format!("squigit-{:032x}.png", rand::random::<u128>()),
                    )
                    .text("folder", "/squigit")
                    .text("isPrivateFile", "true")
                    .text("useUniqueFileName", "true")
                    .text("responseFields", "isPrivateFile"),
            ),
        Provider::Chevereto => client
            .post(custom_endpoint(&config.chevereto_url)?)
            .header("X-API-Key", key.expose())
            .multipart(
                multipart::Form::new()
                    .part("source", file)
                    .text("format", "json")
                    .text("expiration", "PT10M"),
            ),
    };
    let response = request.send().map_err(|error| {
        anyhow::anyhow!(
            "Could not upload the image to {}: {}",
            provider.name(),
            error.without_url()
        )
    })?;
    let body = response_body(response, provider)?;
    match provider {
        Provider::Imgbb => {
            let response: ImgbbResponse =
                serde_json::from_slice(&body).context("ImgBB returned an invalid response")?;
            anyhow::ensure!(
                response.success,
                "{}",
                response
                    .error
                    .and_then(|e| e.message)
                    .unwrap_or_else(|| "ImgBB could not upload the image".into())
                    .replace(key.expose(), "[REDACTED]")
            );
            image_url(
                &response
                    .data
                    .context("ImgBB did not return an image URL")?
                    .url,
            )
        }
        Provider::Imagekit => {
            let response: ImagekitResponse =
                serde_json::from_slice(&body).context("ImageKit returned an invalid response")?;
            let url = image_url(&response.url)?;
            anyhow::ensure!(
                response.is_private_file == Some(true),
                "ImageKit did not confirm the image is private"
            );
            signed_imagekit_url(url, &response.file_path, key)
        }
        Provider::Chevereto => {
            let response: CheveretoResponse =
                serde_json::from_slice(&body).context("Chevereto returned an invalid response")?;
            anyhow::ensure!(
                response.status_code == 200,
                "Chevereto could not upload the image"
            );
            image_url(
                &response
                    .image
                    .context("Chevereto did not return an image URL")?
                    .url,
            )
        }
    }
}

fn response_body(response: Response, provider: Provider) -> Result<Vec<u8>> {
    let status = response.status();
    anyhow::ensure!(
        status.is_success(),
        "{} rejected the upload ({status}). Check its settings in Image hosting.",
        provider.name()
    );
    let mut body = Vec::new();
    response
        .take(1024 * 1024 + 1)
        .read_to_end(&mut body)
        .context("Could not read the image host response")?;
    anyhow::ensure!(
        body.len() <= 1024 * 1024,
        "The image host returned an oversized response"
    );
    Ok(body)
}

fn image_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("The image host returned an invalid URL")?;
    anyhow::ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none(),
        "The image URL must use HTTPS without embedded credentials"
    );
    Ok(url)
}

fn signed_imagekit_url(mut url: Url, file_path: &str, key: &SecretString) -> Result<Url> {
    let mut encoded = Url::parse("https://imagekit.io/")?;
    encoded
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Could not encode the ImageKit file path"))?
        .clear()
        .extend(file_path.split('/').skip(1));
    anyhow::ensure!(
        file_path.starts_with('/') && url.path().ends_with(encoded.path()) && url.query().is_none(),
        "ImageKit returned an unexpected file path"
    );
    let expires =
        (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + LINK_LIFETIME).to_string();
    let mut signer = Hmac::<Sha1>::new_from_slice(key.expose().as_bytes())
        .map_err(|_| anyhow::anyhow!("Could not sign the ImageKit URL"))?;
    signer.update(encoded.path().trim_start_matches('/').as_bytes());
    signer.update(expires.as_bytes());
    let signature = signer
        .finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    url.query_pairs_mut()
        .append_pair("ik-t", &expires)
        .append_pair("ik-s", &signature);
    Ok(url)
}

#[derive(Deserialize)]
struct ImageLink {
    url: String,
}

#[derive(Deserialize)]
struct UploadError {
    message: Option<String>,
}

#[derive(Deserialize)]
struct ImgbbResponse {
    success: bool,
    data: Option<ImageLink>,
    error: Option<UploadError>,
}

#[derive(Deserialize)]
struct ImagekitResponse {
    url: String,
    #[serde(rename = "filePath")]
    file_path: String,
    #[serde(rename = "isPrivateFile")]
    is_private_file: Option<bool>,
}

#[derive(Deserialize)]
struct CheveretoResponse {
    status_code: u16,
    image: Option<ImageLink>,
}
