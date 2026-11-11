use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/runtime.rs"));

pub const VERSION: &str = include_str!("../../../native/ocr/VERSION");

pub fn resolve_sidecar_path() -> Result<PathBuf> {
    anyhow::ensure!(
        !RUNTIME.is_empty(),
        "The embedded OCR runtime is missing; build it with cargo xtask build --ocr and rebuild Squigit"
    );
    let cache = dirs::cache_dir()
        .context("Could not resolve the OCR cache directory")?
        .join("squigit/ocr");
    fs::create_dir_all(&cache)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache.join("extract.lock"))?;
    FileExt::lock_exclusive(&lock).context("Could not lock the embedded OCR runtime")?;

    let runtime = cache.join(RUNTIME_ID);
    let executable = runtime.join(if cfg!(windows) {
        "squigit-ocr.exe"
    } else {
        "squigit-ocr"
    });
    if executable.is_file() && runtime.join(".complete").is_file() {
        return Ok(executable);
    }

    let staging = tempfile::Builder::new()
        .prefix("extract-")
        .tempdir_in(&cache)?;
    tar::Archive::new(GzDecoder::new(RUNTIME))
        .unpack(staging.path())
        .context("Could not extract the embedded OCR runtime")?;
    let name = executable.file_name().unwrap();
    anyhow::ensure!(
        staging.path().join(name).is_file(),
        "The embedded OCR executable is missing"
    );
    fs::write(staging.path().join(".complete"), RUNTIME_ID)?;
    if runtime.exists() {
        fs::remove_dir_all(&runtime)?;
    }
    fs::rename(staging.path(), &runtime).context("Could not install the embedded OCR runtime")?;
    Ok(executable)
}
