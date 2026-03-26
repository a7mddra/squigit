// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::fs;
use xtask::{ocr_sidecar_dir, project_root, qt_native_dir, stt_sidecar_dir, tauri_dir};

pub fn all() -> Result<()> {
    ocr()?;
    stt()?;
    capture()?;
    app()?;
    Ok(())
}

pub fn capture() -> Result<()> {
    println!("\nCleaning capture engine artifacts...");

    let native_dir = qt_native_dir();

    for dir in ["build", "dist"] {
        let path = native_dir.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

pub fn ocr() -> Result<()> {
    println!("\nCleaning OCR sidecar artifacts...");

    let sidecar = ocr_sidecar_dir();

    for dir in ["venv", "build", "dist", "models", "dist_bundled"] {
        let path = sidecar.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

pub fn stt() -> Result<()> {
    println!("\nCleaning STT sidecar artifacts...");

    let sidecar = stt_sidecar_dir();

    for dir in ["build", "models"] {
        let path = sidecar.join(dir);
        if path.exists() {
            println!("  Removing {}", path.display());
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

pub fn app() -> Result<()> {
    println!("\nCleaning Tauri app and packaging binaries...");

    let desktop_binaries = tauri_dir().join("binaries");
    if desktop_binaries.exists() {
        println!("  Removing {}", desktop_binaries.display());
        fs::remove_dir_all(&desktop_binaries)?;
    }

    let pkg_binaries = project_root().join("packaging").join("binaries");
    if pkg_binaries.exists() {
        println!("  Removing {}", pkg_binaries.display());
        fs::remove_dir_all(&pkg_binaries)?;
    }

    let cargo_target = project_root().join("target");
    if cargo_target.exists() {
        println!("  Removing {}", cargo_target.display());
        fs::remove_dir_all(&cargo_target)?;
    }

    let tauri_target = tauri_dir().join("target");
    if tauri_target.exists() {
        println!("  Removing {}", tauri_target.display());
        fs::remove_dir_all(&tauri_target)?;
    }

    Ok(())
}
