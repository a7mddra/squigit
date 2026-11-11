use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

pub struct RuntimePaths {
    pub bin: PathBuf,
    pub env_vars: Vec<(String, String)>,
}

impl RuntimePaths {
    #[cfg(feature = "development")]
    pub fn resolve() -> Result<Self> {
        anyhow::ensure!(
            cfg!(debug_assertions),
            "Development Qt paths require a debug build"
        );
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../native/qt/build-debug/runtime-Debug.txt");
        let configuration = std::fs::read_to_string(&manifest).with_context(|| {
            format!(
                "Could not read {}; run cargo xtask build --debug",
                manifest.display()
            )
        })?;
        let paths = configuration.lines().collect::<Vec<_>>();
        anyhow::ensure!(
            paths.len() == 4 && paths.iter().all(|path| !path.is_empty()),
            "Invalid Qt development paths in {}; run cargo xtask build --debug",
            manifest.display()
        );
        let bin = PathBuf::from(paths[0]);
        let libraries = PathBuf::from(paths[1]);
        let plugins = PathBuf::from(paths[2]);
        let qml = PathBuf::from(paths[3]);
        anyhow::ensure!(
            bin.is_file(),
            "Qt debug executable is missing at {}; run cargo xtask build --debug",
            bin.display()
        );
        anyhow::ensure!(libraries.is_dir() && plugins.is_dir() && qml.is_dir(), "The Qt development kit is incomplete; run cargo xtask build --debug after installing the required Qt modules");
        let mut env_vars = vec![
            (
                "QT_PLUGIN_PATH".to_string(),
                plugins.to_string_lossy().into_owned(),
            ),
            (
                "QT_QPA_PLATFORM_PLUGIN_PATH".to_string(),
                plugins.join("platforms").to_string_lossy().into_owned(),
            ),
            (
                "QML2_IMPORT_PATH".to_string(),
                qml.to_string_lossy().into_owned(),
            ),
        ];
        #[cfg(target_os = "linux")]
        env_vars.push(prepend_path("LD_LIBRARY_PATH", &libraries)?);
        #[cfg(target_os = "windows")]
        env_vars.push(prepend_path("PATH", &libraries)?);
        #[cfg(target_os = "macos")]
        env_vars.push(prepend_path("DYLD_FRAMEWORK_PATH", &libraries)?);
        Ok(Self { bin, env_vars })
    }

    #[cfg(not(feature = "development"))]
    pub fn resolve() -> Result<Self> {
        let exe_path = env::current_exe().context("Could not resolve the squigit executable")?;
        let parent = exe_path
            .parent()
            .context("The squigit executable has no parent directory")?;
        let runtime = parent.join("_internal/qt");

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
                .join("squigit-qt.app")
                .join("Contents")
                .join("MacOS")
                .join("squigit-qt");
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

#[cfg(feature = "development")]
fn prepend_path(name: &str, path: &std::path::Path) -> Result<(String, String)> {
    let mut paths = vec![path.to_path_buf()];
    if let Some(existing) = env::var_os(name) {
        paths.extend(env::split_paths(&existing));
    }
    let value = env::join_paths(paths).with_context(|| format!("Could not set {name} for Qt"))?;
    Ok((name.to_string(), value.to_string_lossy().into_owned()))
}
