use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use squigit_ocr::models::{ModelStorage, DEFAULT_OCR_MODEL_ID};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub ocr: OcrConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct OcrConfig {
    pub selected_model: String,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            selected_model: DEFAULT_OCR_MODEL_ID.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppData {
    root: PathBuf,
}

impl AppData {
    pub fn open() -> Result<Self> {
        let root = std::env::var_os("SQUIGIT_CONFIG_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                dirs::config_dir().map(|directory| {
                    directory.join(if cfg!(target_os = "linux") {
                        "squigit"
                    } else {
                        "Squigit"
                    })
                })
            })
            .context("Could not find the Squigit app data directory")?;
        let data = Self { root };
        data.ensure()?;
        Ok(data)
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn ensure(&self) -> Result<()> {
        self.load().map(|_| ())
    }

    pub fn load(&self) -> Result<Config> {
        self.access(|_| Ok(()))
    }

    pub fn update(&self, update: impl FnOnce(&mut Config)) -> Result<()> {
        self.access(|config| {
            update(config);
            self.save(config)
        })
        .map(|_| ())
    }

    fn access(&self, action: impl FnOnce(&mut Config) -> Result<()>) -> Result<Config> {
        fs::create_dir_all(self.models_dir())
            .with_context(|| format!("Could not create app data at {}", self.root.display()))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.root.join(".config.lock"))?;
        lock.lock_exclusive()?;
        let path = self.config_path();
        let mut config = match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents)
                .with_context(|| format!("Could not read settings from {}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                self.save(&config)?;
                config
            }
            Err(error) => {
                return Err(error).with_context(|| format!("Could not open {}", path.display()));
            }
        };
        action(&mut config)?;
        Ok(config)
    }

    fn save(&self, config: &Config) -> Result<()> {
        let contents = toml::to_string_pretty(config)?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        file.write_all(contents.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(self.config_path())
            .map_err(|error| error.error)
            .context("Could not save Squigit settings")?;
        Ok(())
    }
}

impl ModelStorage for AppData {
    fn models_dir(&self) -> PathBuf {
        AppData::models_dir(self)
    }

    fn ensure(&self) -> Result<()> {
        AppData::ensure(self)
    }

    fn selected_model(&self) -> Result<String> {
        Ok(self.load()?.ocr.selected_model)
    }

    fn select_model(&self, model_id: &str) -> Result<()> {
        self.update(|config| config.ocr.selected_model = model_id.to_string())
    }
}
