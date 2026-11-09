use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

pub struct RuntimePaths {
    pub bin: PathBuf,
    pub env_vars: Vec<(String, String)>,
}

impl RuntimePaths {
    pub fn resolve() -> Result<Self> {
        let exe_path = env::current_exe().context("Could not resolve the squigit executable")?;
        let parent = exe_path
            .parent()
            .context("The squigit executable has no parent directory")?;
        let runtime = parent.join("_internal");

        #[cfg(target_os = "linux")]
        {
            let usr = runtime.join("usr");
            let bin = usr.join("bin").join("squigit-qt");
            let libs = usr.join("lib");
            let plugins = usr.join("plugins");
            let qml = usr.join("qml");

            if !bin.exists() {
                anyhow::bail!("squigit Qt runtime not found at {}", bin.display());
            }

            let mut env_vars = Vec::new();
            let mut ld_path = libs.to_string_lossy().to_string();
            if let Ok(existing) = env::var("LD_LIBRARY_PATH") {
                ld_path = format!("{}:{}", ld_path, existing);
            }
            env_vars.push(("LD_LIBRARY_PATH".to_string(), ld_path));
            env_vars.push((
                "QT_PLUGIN_PATH".to_string(),
                plugins.to_string_lossy().to_string(),
            ));
            env_vars.push((
                "QML2_IMPORT_PATH".to_string(),
                qml.to_string_lossy().to_string(),
            ));
            env_vars.push((
                "QT_QPA_PLATFORM_PLUGIN_PATH".to_string(),
                plugins.join("platforms").to_string_lossy().to_string(),
            ));

            Ok(RuntimePaths { bin, env_vars })
        }

        #[cfg(target_os = "macos")]
        {
            let bin = runtime
                .join("Squigit.app")
                .join("Contents")
                .join("MacOS")
                .join("Squigit");
            if !bin.exists() {
                anyhow::bail!("squigit Qt runtime not found at {}", bin.display());
            }
            Ok(RuntimePaths {
                bin,
                env_vars: vec![],
            })
        }

        #[cfg(target_os = "windows")]
        {
            let bin = runtime.join("squigit-qt.exe");
            if !bin.exists() {
                anyhow::bail!("squigit Qt runtime not found at {}", bin.display());
            }
            Ok(RuntimePaths {
                bin,
                env_vars: vec![],
            })
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("Unsupported platform")
        }
    }
}
