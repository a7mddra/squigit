use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build(root: &Path) -> Result<()> {
    let native = root.join("native/ocr");
    let python = python(&native);
    if !python.is_file() {
        let launcher = std::env::var_os("SQUIGIT_PYTHON").unwrap_or_else(|| {
            if cfg!(windows) {
                "python".into()
            } else {
                "python3".into()
            }
        });
        run(Command::new(launcher)
            .args(["-m", "venv"])
            .arg(native.join(".venv")))?;
    }
    run(Command::new(&python)
        .args(["-m", "pip", "install", "-r", "requirements.txt"])
        .current_dir(&native))?;
    run(Command::new(&python)
        .arg("scripts/prepare_models.py")
        .current_dir(&native))?;
    run(Command::new(&python)
        .args(["-m", "PyInstaller", "--noconfirm", "squigit-ocr.spec"])
        .current_dir(&native))
}

fn python(native: &Path) -> PathBuf {
    native.join(if cfg!(windows) {
        ".venv/Scripts/python.exe"
    } else {
        ".venv/bin/python"
    })
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .context("Could not start OCR build command")?;
    anyhow::ensure!(status.success(), "OCR build command failed: {command:?}");
    Ok(())
}
