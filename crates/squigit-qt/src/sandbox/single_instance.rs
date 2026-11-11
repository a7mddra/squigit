use anyhow::{anyhow, Context, Result};
use fs2::FileExt;
use std::fs::{self, File};

pub struct InstanceLock {
    file: File,
}

impl InstanceLock {
    pub fn try_acquire() -> Result<Self> {
        let dir = dirs::runtime_dir()
            .or_else(dirs::cache_dir)
            .context("Could not find the runtime directory")?;
        fs::create_dir_all(&dir)?;
        let path = dir.join("squigit-qt.lock");
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("Could not open {}", path.display()))?;
        file.try_lock_exclusive()
            .map_err(|_| anyhow!("The Qt runtime is already running"))?;
        Ok(Self { file })
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}
