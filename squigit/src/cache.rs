use anyhow::{Context, Result};
use fs2::FileExt;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::storage::AppData;

fn roots(data: &AppData) -> Result<Vec<PathBuf>> {
    let base = dirs::cache_dir().context("Could not find the cache directory")?;
    let qt = base.join("Squigit/Squigit");
    #[cfg(target_os = "windows")]
    let qt = qt.join("cache");
    Ok(vec![
        qt,
        squigit_ocr::sidecar::runtime_cache_dir()?,
        data.blobs_dir(),
    ])
}

fn entries(path: &Path) -> Result<Vec<PathBuf>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not read {}", path.display()))
        }
    };
    anyhow::ensure!(
        metadata.is_dir() && !metadata.is_symlink(),
        "Storage directory is not a regular folder: {}",
        path.display()
    );
    fs::read_dir(path)?
        .filter_map(|entry| match entry {
            Ok(entry)
                if matches!(
                    entry.file_name().to_str(),
                    Some("usage.lock" | "extract.lock")
                ) =>
            {
                None
            }
            Ok(entry) => Some(Ok(entry.path())),
            Err(error) => Some(Err(error.into())),
        })
        .collect()
}

fn size(path: &Path) -> Result<u64> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if metadata.is_symlink() {
        return Ok(0);
    }
    if metadata.is_dir() {
        return fs::read_dir(path)?.try_fold(0u64, |total, entry| {
            Ok(total.saturating_add(size(&entry?.path())?))
        });
    }
    Ok(metadata.len())
}

pub fn storage_size(data: &AppData) -> Result<u64> {
    let mut total = 0u64;
    for root in roots(data)? {
        for path in entries(&root)? {
            total = total.saturating_add(size(&path)?);
        }
    }
    Ok(total)
}

pub fn clear(data: &AppData) -> Result<()> {
    let blobs = data.blob_lock()?;
    FileExt::try_lock_exclusive(&blobs)
        .context("An image upload is active. Try clearing storage after it finishes.")?;
    let runtime = squigit_ocr::sidecar::runtime_cache_lock()?;
    FileExt::try_lock_exclusive(&runtime)
        .context("OCR is active. Try clearing storage after it finishes.")?;
    let mut failed = Vec::new();
    for root in roots(data)? {
        for path in entries(&root)? {
            let result = match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => {
                    fs::remove_dir_all(&path)
                }
                Ok(_) => fs::remove_file(&path),
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                if error.kind() != std::io::ErrorKind::NotFound {
                    failed.push(format!("{}: {error}", path.display()));
                }
            }
        }
    }
    anyhow::ensure!(
        failed.is_empty(),
        "Some files could not be cleared: {}",
        failed.join("; ")
    );
    Ok(())
}
