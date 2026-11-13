use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(repo_root: &Path) -> Result<(), String> {
    crate::ocr::run(repo_root, false)?;
    crate::qt::release(repo_root)?;
    app(repo_root)
}

pub fn app(repo_root: &Path) -> Result<(), String> {
    app_inner(repo_root).map_err(|error| format!("{error:#}"))
}

pub fn debug(repo_root: &Path) -> Result<(), String> {
    debug_inner(repo_root).map_err(|error| format!("{error:#}"))
}

fn debug_inner(repo_root: &Path) -> Result<()> {
    let host = host_target_triple()?;
    let ocr = repo_root
        .join("binaries")
        .join(format!("paddle-ocr-{host}"))
        .join(if cfg!(windows) {
            "squigit-ocr.exe"
        } else {
            "squigit-ocr"
        });
    anyhow::ensure!(
        ocr.is_file(),
        "The complete OCR runtime is required at {}. Run cargo xtask build --ocr once; debug builds never rebuild Python.",
        ocr.display()
    );
    crate::qt::debug(repo_root).map_err(anyhow::Error::msg)?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run_command(
        Command::new(cargo)
            .args(["build", "-p", "squigit", "--features", "development"])
            .current_dir(repo_root),
        "cargo build -p squigit --features development",
    )?;
    println!("\nDebug application ready. Run cargo xtask run.");
    Ok(())
}

fn app_inner(repo_root: &Path) -> Result<()> {
    anyhow::ensure!(
        repo_root.join("native/qt/_internal").is_dir(),
        "The staged Qt runtime is missing; run cargo xtask stage-qt first"
    );
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    run_command(
        Command::new(cargo)
            .args(["build", "--release", "-p", "squigit"])
            .current_dir(repo_root),
        "cargo build --release -p squigit",
    )?;
    package(repo_root)
}

fn package(repo_root: &Path) -> Result<()> {
    let destination = repo_root.join("binaries").join(host_target_triple()?);
    remove_existing_path(&destination)?;
    fs::create_dir_all(&destination)?;
    crate::qt::package(repo_root, &destination)?;
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

    copy_app_files(repo_root, &destination)
}

fn copy_app_files(repo_root: &Path, destination: &Path) -> Result<()> {
    fs::copy(
        repo_root.join("squigit/icons/icon.png"),
        destination.join("squigit.png"),
    )?;
    #[cfg(target_os = "linux")]
    {
        fs::copy(
            repo_root.join("squigit/squigit.desktop"),
            destination.join("squigit.desktop"),
        )?;
        let policy = destination.join("polkit");
        fs::create_dir_all(&policy)?;
        for name in ["org.squigit.policy", "install-policy.sh"] {
            fs::copy(
                repo_root.join("squigit/resources/polkit").join(name),
                policy.join(name),
            )?;
        }
    }
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
