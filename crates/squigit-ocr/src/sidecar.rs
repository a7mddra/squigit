use anyhow::{Context, Result};
#[cfg(not(feature = "staged-runtime"))]
use flate2::read::GzDecoder;
use fs2::FileExt;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/runtime.rs"));

pub const VERSION: &str = include_str!("../../../native/ocr/VERSION");

pub fn runtime_cache_dir() -> Result<PathBuf> {
    dirs::cache_dir()
        .context("Could not resolve the OCR cache directory")
        .map(|directory| directory.join("squigit/ocr"))
}

pub fn runtime_cache_lock() -> Result<fs::File> {
    let directory = runtime_cache_dir()?;
    fs::create_dir_all(&directory)?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("usage.lock"))
        .context("Could not lock the OCR cache")
}

pub fn resolve_sidecar_path() -> Result<PathBuf> {
    acquire_runtime().map(|(path, _)| path)
}

pub fn acquire_runtime() -> Result<(PathBuf, fs::File)> {
    let lease = runtime_cache_lock()?;
    FileExt::lock_shared(&lease)?;
    Ok((resolve_runtime()?, lease))
}

#[cfg(feature = "staged-runtime")]
fn resolve_runtime() -> Result<PathBuf> {
    let executable = PathBuf::from(STAGED_RUNTIME).join(if cfg!(windows) {
        "squigit-ocr.exe"
    } else {
        "squigit-ocr"
    });
    anyhow::ensure!(
        executable.is_file(),
        "The complete staged OCR runtime is required at {}; run cargo xtask build --ocr",
        executable.display()
    );
    Ok(executable)
}

#[cfg(not(feature = "staged-runtime"))]
fn resolve_runtime() -> Result<PathBuf> {
    anyhow::ensure!(
        !RUNTIME.is_empty(),
        "The embedded OCR runtime is missing; build it with cargo xtask build --ocr and rebuild Squigit"
    );
    let cache = runtime_cache_dir()?;
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
