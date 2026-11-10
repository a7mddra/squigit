use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(repo_root: &Path, native_only: bool) -> Result<(), String> {
    run_inner(repo_root, native_only).map_err(|error| format!("{error:#}"))
}

fn run_inner(repo_root: &Path, native_only: bool) -> Result<()> {
    let native_dir = repo_root.join("native/qt");

    if native_only {
        println!("\nBuilding Squigit Qt runtime (CMake only)...");
        build_native(&native_dir)?;
        println!("\nSquigit Qt runtime build complete!");
        return Ok(());
    }

    println!("\nBuilding Squigit...");
    println!("\nRunning Qt CMake build...");
    build_native(&native_dir)?;

    println!("\nDeploying Qt runtime...");
    deploy(&native_dir)?;

    #[cfg(target_os = "macos")]
    {
        println!("\nSigning macOS bundle...");
        macos::sign(&native_dir)?;
    }

    println!("\nBuilding Rust binary...");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run_command(
        Command::new(cargo)
            .args(["build", "--release", "-p", "squigit"])
            .current_dir(repo_root),
        "cargo build --release -p squigit",
    )?;

    println!("\nPackaging Squigit...");
    package(repo_root, &native_dir)?;
    println!("\nSquigit build complete!");
    Ok(())
}

#[cfg(target_os = "linux")]
fn build_native(native_dir: &Path) -> Result<()> {
    linux::build(native_dir)
}

#[cfg(target_os = "macos")]
fn build_native(native_dir: &Path) -> Result<()> {
    macos::build(native_dir)
}

#[cfg(target_os = "windows")]
fn build_native(native_dir: &Path) -> Result<()> {
    windows::build(native_dir)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn build_native(_native_dir: &Path) -> Result<()> {
    Err(anyhow::anyhow!(
        "Squigit Qt runtime builds are unsupported on this operating system"
    ))
}

#[cfg(target_os = "linux")]
fn deploy(native_dir: &Path) -> Result<()> {
    linux::deploy(native_dir)
}

#[cfg(target_os = "macos")]
fn deploy(native_dir: &Path) -> Result<()> {
    macos::deploy(native_dir)
}

#[cfg(target_os = "windows")]
fn deploy(native_dir: &Path) -> Result<()> {
    windows::deploy(native_dir)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn deploy(_native_dir: &Path) -> Result<()> {
    Err(anyhow::anyhow!(
        "Squigit Qt runtime deployment is unsupported on this operating system"
    ))
}

fn package(repo_root: &Path, native_dir: &Path) -> Result<()> {
    let host = host_target_triple()?;
    let destination = repo_root.join("binaries").join(host);
    remove_existing_path(&destination)?;
    fs::create_dir_all(&destination)
        .with_context(|| format!("Could not create {}", destination.display()))?;

    let runtime_source = native_dir.join("_internal");
    if !runtime_source.exists() {
        anyhow::bail!("Qt runtime not found at {}", runtime_source.display());
    }

    let runtime_destination = destination.join("_internal");
    println!("  Moving Qt runtime to {}", runtime_destination.display());
    if fs::rename(&runtime_source, &runtime_destination).is_err() {
        copy_runtime(&runtime_source, &runtime_destination)?;
        fs::remove_dir_all(&runtime_source).with_context(|| {
            format!(
                "Could not remove staged runtime {}",
                runtime_source.display()
            )
        })?;
    }

    let binary_name = if cfg!(windows) {
        "squigit.exe"
    } else {
        "squigit"
    };
    let binary_source = repo_root.join("target/release").join(binary_name);
    if !binary_source.is_file() {
        anyhow::bail!("Rust binary not found: {}", binary_source.display());
    }
    let binary_destination = destination.join(binary_name);
    println!(
        "  Copying squigit binary to {}",
        binary_destination.display()
    );
    fs::copy(&binary_source, &binary_destination).with_context(|| {
        format!(
            "Could not copy {} to {}",
            binary_source.display(),
            binary_destination.display()
        )
    })?;

    Ok(())
}

pub fn host_target_triple() -> Result<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("Could not inspect the Rust host target")?;
    if !output.status.success() {
        anyhow::bail!("Could not inspect the Rust host target with 'rustc -vV'");
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::trim))
        .filter(|host| !host.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("Rust did not report a host target in 'rustc -vV'"))
}

fn run_command(command: &mut Command, display: &str) -> Result<()> {
    println!("  $ {display}");
    let status = command
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to run: {display}"))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("'{display}' failed with exit code {:?}", status.code())
    }
}

fn remove_existing_path(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            fs::remove_file(path)
                .with_context(|| format!("Could not remove {}", path.display()))?;
        }
        Ok(_) => fs::remove_dir_all(path)
            .with_context(|| format!("Could not remove {}", path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("Could not inspect {}", path.display()));
        }
    }
    Ok(())
}

#[cfg(windows)]
#[allow(dead_code)]
fn copy_runtime(source: &Path, destination: &Path) -> Result<()> {
    copy_directory(source, destination, false)
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn copy_runtime(source: &Path, destination: &Path) -> Result<()> {
    copy_directory(source, destination, true)
}

#[allow(dead_code)]
fn copy_directory(source: &Path, destination: &Path, preserve_symlinks: bool) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("Could not create {}", destination.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("Could not read {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if preserve_symlinks && file_type.is_symlink() {
            remove_existing_path(&destination_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(&source_path)?, &destination_path)?;
            #[cfg(not(unix))]
            fs::copy(&source_path, &destination_path)?;
        } else if file_type.is_dir() {
            copy_directory(&source_path, &destination_path, preserve_symlinks)?;
        } else {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "Could not copy {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux {
    use anyhow::{Context, Result};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;

    pub fn build(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        println!("  Configuring CMake...");
        fs::create_dir_all(&build_dir)?;

        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("-S")
            .arg(native_dir)
            .arg("-B")
            .arg(&build_dir)
            .arg("-DCMAKE_BUILD_TYPE=Release");

        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            cmake_cmd.arg(format!("-DCMAKE_PREFIX_PATH={qt6_dir}"));
        } else if let Ok(prefix) = std::env::var("CMAKE_PREFIX_PATH") {
            cmake_cmd.arg(format!("-DCMAKE_PREFIX_PATH={prefix}"));
        }

        let status = cmake_cmd
            .status()
            .context("Failed to run cmake configure")?;
        if !status.success() {
            anyhow::bail!("CMake configure failed");
        }

        println!("  Building...");
        let status = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .args(["--config", "Release", "--parallel"])
            .status()
            .context("Failed to run cmake build")?;
        if !status.success() {
            anyhow::bail!("CMake build failed");
        }
        Ok(())
    }

    pub fn deploy(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        let runtime_dir = native_dir.join("_internal");
        println!("  Creating '_internal' distribution using linuxdeployqt...");
        create_runtime_distribution(native_dir, &build_dir, &runtime_dir)
    }

    fn create_runtime_distribution(
        native_dir: &Path,
        build_dir: &Path,
        runtime_dir: &Path,
    ) -> Result<()> {
        if runtime_dir.exists() {
            fs::remove_dir_all(runtime_dir)?;
        }
        let bin_dir = runtime_dir.join("usr/bin");
        fs::create_dir_all(&bin_dir)?;

        let source_binary = build_dir.join("squigit-qt");
        let destination_binary = bin_dir.join("squigit-qt");
        if !source_binary.exists() {
            anyhow::bail!("Compiled binary not found at {}", source_binary.display());
        }
        fs::copy(&source_binary, &destination_binary)?;
        fs::set_permissions(&destination_binary, fs::Permissions::from_mode(0o755))?;

        let qmake_path = resolve_qmake_path();
        println!("  Using qmake: {qmake_path}");
        let qml_dir = native_dir.join("ui");
        if !qml_dir.exists() {
            anyhow::bail!("QML source directory not found at {}", qml_dir.display());
        }

        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            let sql_drivers = Path::new(&qt6_dir).join("plugins/sqldrivers");
            if sql_drivers.exists() {
                for plugin in [
                    "libqsqlmimer.so",
                    "libqsqlmysql.so",
                    "libqsqlodbc.so",
                    "libqsqlpsql.so",
                ] {
                    let plugin_path = sql_drivers.join(plugin);
                    if plugin_path.exists() {
                        println!("  Removing problematic SQL plugin: {plugin}");
                        let _ = fs::remove_file(plugin_path);
                    }
                }
            }
        }

        let mut command = Command::new("linuxdeployqt");
        command.arg(&destination_binary).args([
            "-bundle-non-qt-libs",
            "-always-overwrite",
            "-verbose=2",
            "-unsupported-allow-new-glibc",
            &format!("-qmake={qmake_path}"),
            &format!("-qmldir={}", qml_dir.display()),
        ]);
        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            let qt_library_path = Path::new(&qt6_dir).join("lib");
            if qt_library_path.exists() {
                println!(
                    "  Setting LD_LIBRARY_PATH to include: {}",
                    qt_library_path.display()
                );
                let current = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
                let value = if current.is_empty() {
                    qt_library_path.to_string_lossy().into_owned()
                } else {
                    format!("{}:{current}", qt_library_path.display())
                };
                command.env("LD_LIBRARY_PATH", value);
            }
        }

        let status = command
            .status()
            .context("Failed to execute linuxdeployqt")?;
        if !status.success() {
            anyhow::bail!("linuxdeployqt failed to bundle the application");
        }
        println!(
            "  Success! Portable runtime created at: {}",
            runtime_dir.display()
        );
        Ok(())
    }

    fn resolve_qmake_path() -> String {
        if let Ok(path) = std::env::var("QMAKE") {
            return path;
        }
        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            let qmake_candidate = Path::new(&qt6_dir).join("bin/qmake");
            if qmake_candidate.exists() {
                return qmake_candidate.to_string_lossy().into_owned();
            }
        }
        if which::which("qmake6").is_ok() {
            return "qmake6".to_string();
        }
        if which::which("qmake-qt6").is_ok() {
            return "qmake-qt6".to_string();
        }
        if let Ok(output) = Command::new("qmake").arg("-v").output() {
            if String::from_utf8_lossy(&output.stdout).contains("Qt version 6") {
                return "qmake".to_string();
            }
        }
        "qmake".to_string()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::copy_directory;
    use anyhow::{Context, Result};
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    pub fn build(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        let qt_prefix = find_qt_prefix()?;
        println!("  Qt Prefix: {qt_prefix}");

        println!("  Configuring CMake...");
        fs::create_dir_all(&build_dir)?;
        let status = Command::new("cmake")
            .arg("-S")
            .arg(native_dir)
            .arg("-B")
            .arg(&build_dir)
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg(format!("-DCMAKE_PREFIX_PATH={qt_prefix}"))
            .status()
            .context("Failed to run cmake configure")?;
        if !status.success() {
            anyhow::bail!("CMake configure failed");
        }

        println!("  Building...");
        let status = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .args(["--config", "Release", "--parallel"])
            .status()
            .context("Failed to run cmake build")?;
        if !status.success() {
            anyhow::bail!("CMake build failed");
        }
        Ok(())
    }

    pub fn deploy(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        let distribution_dir = native_dir.join("_internal");
        let qt_prefix = find_qt_prefix()?;
        println!("  Running macdeployqt...");
        create_distribution(&build_dir, &distribution_dir, &qt_prefix)
    }

    pub fn sign(native_dir: &Path) -> Result<()> {
        println!("  Signing bundle...");
        let app_bundle = native_dir.join("_internal/Squigit.app");
        if !app_bundle.exists() {
            anyhow::bail!("App bundle not found at {}", app_bundle.display());
        }
        let signing_identity = std::env::var("APPLE_SIGNING_IDENTITY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string());
        let status = Command::new("codesign")
            .args([
                "-s",
                signing_identity.as_str(),
                "--deep",
                "--force",
                "--options",
                "runtime",
            ])
            .arg(&app_bundle)
            .status()
            .context("Failed to execute codesign")?;
        if !status.success() {
            anyhow::bail!("Code signing failed");
        }
        Ok(())
    }

    fn find_qt_prefix() -> Result<String> {
        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            if Path::new(&qt6_dir).exists() {
                return Ok(qt6_dir);
            }
        }
        if let Ok(prefix) = std::env::var("CMAKE_PREFIX_PATH") {
            for path in std::env::split_paths(&prefix) {
                if path.exists() {
                    return Ok(path.to_string_lossy().into_owned());
                }
            }
        }
        for candidate in [
            "/opt/homebrew/opt/qt@6",
            "/usr/local/opt/qt@6",
            "/opt/qt/6.6.0/macos",
            "/opt/qt/6.6.0/clang_64",
        ] {
            if Path::new(candidate).exists() {
                return Ok(candidate.to_string());
            }
        }
        if let Ok(output) = Command::new("qmake6")
            .args(["-query", "QT_INSTALL_PREFIX"])
            .output()
        {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !prefix.is_empty() {
                    return Ok(prefix);
                }
            }
        }
        anyhow::bail!("Qt6 not found. Install with: brew install qt@6")
    }

    fn create_distribution(
        build_dir: &Path,
        distribution_dir: &Path,
        qt_prefix: &str,
    ) -> Result<()> {
        if distribution_dir.exists() {
            fs::remove_dir_all(distribution_dir)?;
        }
        fs::create_dir_all(distribution_dir)?;

        let app_source = build_dir.join("Squigit.app");
        let app_destination = distribution_dir.join("Squigit.app");
        if !app_source.exists() {
            anyhow::bail!("Built app not found: {}", app_source.display());
        }
        copy_directory(&app_source, &app_destination, false)?;

        let macdeployqt = Path::new(qt_prefix).join("bin/macdeployqt");
        if macdeployqt.exists() {
            let status = Command::new(&macdeployqt)
                .arg(&app_destination)
                .status()
                .context("Failed to run macdeployqt")?;
            if !status.success() {
                println!("  Warning: macdeployqt failed, continuing anyway");
            }
        } else {
            println!(
                "  Warning: macdeployqt not found at {}",
                macdeployqt.display()
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use anyhow::{Context, Result};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub fn build(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        let qt_path = find_qt_path()?;
        println!("  Qt Path: {qt_path}");

        println!("  Configuring CMake...");
        let Some(generator) = find_visual_studio_generator() else {
            anyhow::bail!(
                "No compatible Visual Studio C++ toolchain was detected. Install Visual Studio Build Tools with Desktop development with C++."
            );
        };
        println!("  Using CMake generator '{generator}'.");
        let configured =
            run_cmake_configure(native_dir, &build_dir, &qt_path, generator, &["-A", "x64"])?;
        if !configured {
            anyhow::bail!("CMake configure failed");
        }

        println!("  Building...");
        let status = Command::new("cmake")
            .arg("--build")
            .arg(&build_dir)
            .args(["--config", "Release", "--parallel"])
            .status()
            .context("Failed to run cmake build")?;
        if !status.success() {
            anyhow::bail!("CMake build failed");
        }
        Ok(())
    }

    fn run_cmake_configure(
        native_dir: &Path,
        build_dir: &Path,
        qt_path: &str,
        generator: &str,
        extra_args: &[&str],
    ) -> Result<bool> {
        if build_dir.exists() {
            fs::remove_dir_all(build_dir)?;
        }
        fs::create_dir_all(build_dir)?;

        let mut arguments = vec![
            "-S".to_string(),
            native_dir.to_string_lossy().to_string(),
            "-B".to_string(),
            build_dir.to_string_lossy().to_string(),
            "-G".to_string(),
            generator.to_string(),
            "-DCMAKE_BUILD_TYPE=Release".to_string(),
            format!("-DCMAKE_PREFIX_PATH={qt_path}"),
        ];
        arguments.extend(extra_args.iter().map(|argument| argument.to_string()));
        let status = Command::new("cmake")
            .args(&arguments)
            .status()
            .context("Failed to run cmake configure")?;
        Ok(status.success())
    }

    fn find_visual_studio_generator() -> Option<&'static str> {
        [
            ("Visual Studio 18 2026", "[18.0,19.0)"),
            ("Visual Studio 17 2022", "[17.0,18.0)"),
            ("Visual Studio 16 2019", "[16.0,17.0)"),
        ]
        .into_iter()
        .find_map(|(generator, range)| has_visual_studio_msvc(range).then_some(generator))
    }

    fn has_visual_studio_msvc(version_range: &str) -> bool {
        let vswhere =
            Path::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
        if !vswhere.exists() {
            return false;
        }
        Command::new(vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-version",
                version_range,
                "-property",
                "installationPath",
            ])
            .output()
            .map(|output| {
                output.status.success()
                    && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
            })
            .unwrap_or(false)
    }

    pub fn deploy(native_dir: &Path) -> Result<()> {
        let build_dir = native_dir.join("build");
        let distribution_dir = native_dir.join("_internal");
        let qt_path = find_qt_path()?;
        println!("  Running windeployqt...");
        create_distribution(native_dir, &build_dir, &distribution_dir, &qt_path)
    }

    fn find_qt_path() -> Result<String> {
        let mut candidates = Vec::new();
        if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
            candidates.push(normalize_qt_prefix(PathBuf::from(qt6_dir)));
        }
        if let Ok(qt_dir) = std::env::var("QTDIR") {
            candidates.push(PathBuf::from(qt_dir));
        }
        if let Ok(prefix_path) = std::env::var("CMAKE_PREFIX_PATH") {
            candidates.extend(std::env::split_paths(&prefix_path).map(normalize_qt_prefix));
        }
        if let Ok(output) = Command::new("qmake")
            .args(["-query", "QT_INSTALL_PREFIX"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    candidates.push(PathBuf::from(path));
                }
            }
        }

        candidates.extend(
            [
                r"C:\Qt\6.8.0\msvc2022_64",
                r"C:\Qt\6.8.0\msvc2019_64",
                r"C:\Qt\6.7.0\msvc2022_64",
                r"C:\Qt\6.7.0\msvc2019_64",
                r"C:\Qt\6.6.0\msvc2022_64",
                r"C:\Qt\6.6.0\msvc2019_64",
            ]
            .into_iter()
            .map(PathBuf::from),
        );

        if let Some(home) = dirs::home_dir() {
            for version in ["6.8.0", "6.7.0", "6.6.0"] {
                candidates.push(home.join(version).join("msvc2022_64"));
                candidates.push(home.join(version).join("msvc2019_64"));
                candidates.push(home.join("Qt").join(version).join("msvc2022_64"));
                candidates.push(home.join("Qt").join(version).join("msvc2019_64"));
            }
        }

        let mut attempted = Vec::new();
        for candidate in dedupe_existing_paths(candidates) {
            attempted.push(candidate.display().to_string());
            if qt_has_required_modules(&candidate) {
                return Ok(candidate.to_string_lossy().to_string());
            }
        }
        anyhow::bail!(
            "Qt6 with required modules not found. Checked: {}. Install Qt Declarative (Qt Quick/QML) and Qt5Compat.GraphicalEffects for this kit, or set Qt6_DIR to a full Qt kit root.",
            attempted.join(", ")
        )
    }

    fn normalize_qt_prefix(path: PathBuf) -> PathBuf {
        let is_qt6_cmake_dir = path
            .file_name()
            .map(|name| name.to_string_lossy().eq_ignore_ascii_case("Qt6"))
            .unwrap_or(false)
            && path.join("Qt6Config.cmake").exists();
        if is_qt6_cmake_dir {
            if let Some(prefix) = path.parent().and_then(Path::parent).and_then(Path::parent) {
                return prefix.to_path_buf();
            }
        }
        path
    }

    fn dedupe_existing_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut output = Vec::new();
        for path in paths {
            if path.exists() && !output.iter().any(|existing| existing == &path) {
                output.push(path);
            }
        }
        output
    }

    fn qt_has_required_modules(prefix: &Path) -> bool {
        [
            prefix.join(r"lib\cmake\Qt6\Qt6Config.cmake"),
            prefix.join(r"lib\cmake\Qt6Quick\Qt6QuickConfig.cmake"),
            prefix.join(r"lib\cmake\Qt6Qml\Qt6QmlConfig.cmake"),
            prefix.join(r"qml\Qt5Compat\GraphicalEffects\qmldir"),
        ]
        .iter()
        .all(|path| path.exists())
    }

    fn create_distribution(
        native_dir: &Path,
        build_dir: &Path,
        distribution_dir: &Path,
        qt_path: &str,
    ) -> Result<()> {
        if distribution_dir.exists() {
            fs::remove_dir_all(distribution_dir)?;
        }
        fs::create_dir_all(distribution_dir)?;

        let executable_name = "squigit-qt.exe";
        let executable_source = build_dir.join(executable_name);
        let executable_destination = distribution_dir.join(executable_name);
        if executable_source.exists() {
            fs::copy(&executable_source, &executable_destination)?;
        } else {
            let release_executable = build_dir.join("Release").join(executable_name);
            if !release_executable.exists() {
                anyhow::bail!("Built exe not found: {}", executable_source.display());
            }
            fs::copy(release_executable, &executable_destination)?;
        }

        let windeployqt = Path::new(qt_path).join("bin/windeployqt.exe");
        if !windeployqt.exists() {
            anyhow::bail!("windeployqt not found at {}", windeployqt.display());
        }
        let qml_dir = native_dir.join("ui");
        let status = Command::new(&windeployqt)
            .current_dir(distribution_dir)
            .arg(executable_name)
            .args(["--release", "--qmldir"])
            .arg(&qml_dir)
            .args([
                "--compiler-runtime",
                "--no-translations",
                "--no-opengl-sw",
                "--no-system-d3d-compiler",
            ])
            .status()
            .context("Failed to run windeployqt")?;
        if !status.success() {
            println!("  Warning: windeployqt failed");
        }

        ensure_qt5compat_graphicaleffects(distribution_dir, qt_path)?;
        bundle_vc_runtime(distribution_dir)
    }

    fn ensure_qt5compat_graphicaleffects(distribution_dir: &Path, qt_path: &str) -> Result<()> {
        let qt_qml_root = Path::new(qt_path).join("qml");
        let source = qt_qml_root.join("Qt5Compat");
        if !source.join("GraphicalEffects/qmldir").exists() {
            anyhow::bail!(
                "Missing Qt module 'Qt5Compat.GraphicalEffects' under {}.\nInstall Qt5Compat for this Qt kit (Qt Maintenance Tool or aqt module 'qt5compat').",
                qt_qml_root.display()
            );
        }
        copy_dir_recursive(&source, &distribution_dir.join("qml/Qt5Compat"))
    }

    fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                copy_dir_recursive(&source_path, &destination_path)?;
            } else if file_type.is_file() {
                fs::copy(source_path, destination_path)?;
            }
        }
        Ok(())
    }

    fn bundle_vc_runtime(distribution_dir: &Path) -> Result<()> {
        let system32 = std::env::var("SystemRoot")
            .map(|root| Path::new(&root).join("System32"))
            .unwrap_or_else(|_| Path::new(r"C:\Windows\System32").to_path_buf());
        for dll in [
            "vcruntime140.dll",
            "vcruntime140_1.dll",
            "msvcp140.dll",
            "msvcp140_1.dll",
            "ucrtbase.dll",
        ] {
            let source = system32.join(dll);
            let destination = distribution_dir.join(dll);
            if source.exists() && !destination.exists() {
                fs::copy(source, destination)?;
            }
        }
        Ok(())
    }
}
