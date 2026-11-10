use anyhow::{Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

pub fn parse_url(value: &str) -> Result<Url> {
    let url = Url::parse(value.trim()).context("QR code does not contain a valid web URL")?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "QR code does not contain an HTTP or HTTPS URL"
    );
    Ok(url)
}

pub fn scan(path: &Path, cancelled: &AtomicBool) -> Result<Option<String>> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(None);
    }
    let image = image::open(path)
        .context("Could not read the selected image for QR scanning")?
        .into_luma8();
    if cancelled.load(Ordering::Acquire) {
        return Ok(None);
    }
    let mut image = rqrr::PreparedImage::prepare(image);
    let mut decoded = false;
    for grid in image.detect_grids() {
        if cancelled.load(Ordering::Acquire) {
            return Ok(None);
        }
        if let Ok((_, content)) = grid.decode() {
            decoded = true;
            if let Ok(url) = parse_url(&content) {
                return Ok(Some(url.into()));
            }
        }
    }
    if decoded {
        anyhow::bail!("QR code does not contain an HTTP or HTTPS URL");
    }
    anyhow::bail!("No QR code found in the selected region");
}
