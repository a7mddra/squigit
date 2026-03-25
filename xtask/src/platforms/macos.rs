// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

//! macOS Qt deployment.
//!
//! Builds Qt project with CMake and uses macdeployqt for bundling.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use xtask::copy_dir_all;

pub fn build(native_dir: &Path) -> Result<()> {
    let build_dir = native_dir.join("build");

    let qt_prefix = find_qt_prefix()?;
    println!("  Qt Prefix: {}", qt_prefix);

    println!("  Configuring CMake...");
    fs::create_dir_all(&build_dir)?;

    let status = Command::new("cmake")
        .args([
            "-S",
            native_dir.to_str().unwrap(),
            "-B",
            build_dir.to_str().unwrap(),
            "-DCMAKE_BUILD_TYPE=Release",
            &format!("-DCMAKE_PREFIX_PATH={}", qt_prefix),
        ])
        .status()
        .context("Failed to run cmake configure")?;

    if !status.success() {
        anyhow::bail!("CMake configure failed");
    }

    println!("  Building...");
    let status = Command::new("cmake")
        .args([
            "--build",
            build_dir.to_str().unwrap(),
            "--config",
            "Release",
            "--parallel",
        ])
        .status()
        .context("Failed to run cmake build")?;

    if !status.success() {
        anyhow::bail!("CMake build failed");
    }

    Ok(())
}

pub fn deploy(native_dir: &Path) -> Result<()> {
    let build_dir = native_dir.join("build");
    let dist_dir = native_dir.join("_internal");

    let qt_prefix = find_qt_prefix()?;

    println!("  Running macdeployqt...");
    create_distribution(&build_dir, &dist_dir, &qt_prefix)?;

    Ok(())
}

pub fn sign(native_dir: &Path) -> Result<()> {
    println!("  Signing bundle...");

    let app_bundle = native_dir.join("_internal").join("capture.app");

    if !app_bundle.exists() {
        anyhow::bail!("App bundle not found at {}", app_bundle.display());
    }

    let signing_identity = std::env::var("APPLE_SIGNING_IDENTITY")
        .ok()
        .filter(|v| !v.trim().is_empty())
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
    let candidates = [
        "/opt/homebrew/opt/qt@6",
        "/usr/local/opt/qt@6",
        "/opt/qt/6.6.0/macos",
        "/opt/qt/6.6.0/clang_64",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Ok(candidate.to_string());
        }
    }

    if let Ok(output) = Command::new("qmake6")
        .args(["-query", "QT_INSTALL_PREFIX"])
        .output()
    {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    anyhow::bail!("Qt6 not found. Install with: brew install qt@6")
}

fn create_distribution(build_dir: &Path, dist_dir: &Path, qt_prefix: &str) -> Result<()> {
    if dist_dir.exists() {
        fs::remove_dir_all(dist_dir)?;
    }
    fs::create_dir_all(dist_dir)?;

    let app_name = "capture.app";
    let app_src = build_dir.join(app_name);
    let app_dst = dist_dir.join(app_name);

    if !app_src.exists() {
        anyhow::bail!("Built app not found: {}", app_src.display());
    }

    copy_dir_all(&app_src, &app_dst)?;

    let macdeployqt = Path::new(qt_prefix).join("bin").join("macdeployqt");

    if macdeployqt.exists() {
        let status = Command::new(&macdeployqt)
            .arg(&app_dst)
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
