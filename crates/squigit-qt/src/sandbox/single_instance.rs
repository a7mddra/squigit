use anyhow::{anyhow, Context, Result};
use fs2::FileExt;
use std::fs::{self, File};
use std::path::PathBuf;

const LOCK_NAME: &str = "squigit-qt";

pub struct InstanceLock {
    file: File,
    path: PathBuf,
}

impl InstanceLock {
    pub fn try_acquire() -> Result<Self> {
        let dir = Self::lock_dir()?;
        fs::create_dir_all(&dir)?;

        let path = dir.join(format!("{LOCK_NAME}.lock"));

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("Failed to open lock file: {:?}", path))?;

        file.try_lock_exclusive()
            .map_err(|_| anyhow!("Another instance is already running (lock: {:?})", path))?;

        Ok(Self { file, path })
    }

    #[allow(dead_code)]
    pub fn force_release() -> Result<()> {
        let dir = Self::lock_dir()?;
        let path = dir.join(format!("{LOCK_NAME}.lock"));

        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove stale lock: {:?}", path))?;
        }
        Ok(())
    }

    fn lock_dir() -> Result<PathBuf> {
        dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .context("Failed to resolve lock directory (no XDG_RUNTIME_DIR or cache dir)")
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_lifecycle() {
        let lock = InstanceLock::try_acquire();
        assert!(lock.is_ok(), "First lock should succeed");
        assert!(
            InstanceLock::try_acquire().is_err(),
            "Second lock should fail"
        );

        drop(lock);
        assert!(
            InstanceLock::try_acquire().is_ok(),
            "Lock after release should succeed"
        );

        let _lock = InstanceLock::try_acquire().unwrap();
        let result = InstanceLock::force_release();
        assert!(result.is_ok());
    }
}
