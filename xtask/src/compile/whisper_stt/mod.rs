// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use xtask::{project_root, run_cmd};

pub fn build() -> Result<()> {
    println!("\nBuilding Whisper STT sidecar...");
    let sidecar = xtask::stt_sidecar_dir();
    let build_dir = sidecar.join("build");

    refresh_stt_cmake_cache_if_stale(&sidecar, &build_dir)?;
    fs::create_dir_all(&build_dir)?;

    let source_dir = sidecar.to_string_lossy().to_string();
    let build_dir_str = build_dir.to_string_lossy().to_string();

    println!("\nRunning CMake config...");
    run_cmd(
        "cmake",
        &[
            "-S",
            &source_dir,
            "-B",
            &build_dir_str,
            "-DCMAKE_BUILD_TYPE=Release",
        ],
        &project_root(),
    )?;

    println!("\nRunning CMake build...");
    run_cmd(
        "cmake",
        &["--build", &build_dir_str, "--config", "Release"],
        &project_root(),
    )?;

    println!("\nSidecar build complete!");
    crate::packaging::whisper_stt::stt()?;
    Ok(())
}

fn refresh_stt_cmake_cache_if_stale(sidecar: &Path, build_dir: &Path) -> Result<()> {
    let cache_path = build_dir.join("CMakeCache.txt");
    if !cache_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&cache_path).with_context(|| {
        format!(
            "Failed reading Whisper cache file for validation: {}",
            cache_path.display()
        )
    })?;

    let mut cached_home_dir = None;
    let mut cached_cache_dir = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=") {
            cached_home_dir = Some(value.trim().to_string());
        }
        if let Some(value) = line.strip_prefix("CMAKE_CACHEFILE_DIR:INTERNAL=") {
            cached_cache_dir = Some(value.trim().to_string());
        }
    }

    let expected_home = normalize_path(
        &sidecar
            .canonicalize()
            .unwrap_or_else(|_| sidecar.to_path_buf()),
    );
    let expected_cache = normalize_path(
        &build_dir
            .canonicalize()
            .unwrap_or_else(|_| build_dir.to_path_buf()),
    );

    let home_mismatch = cached_home_dir
        .as_deref()
        .map(normalize_path_str)
        .map(|value| value != expected_home)
        .unwrap_or(false);

    let cache_mismatch = cached_cache_dir
        .as_deref()
        .map(normalize_path_str)
        .map(|value| value != expected_cache)
        .unwrap_or(false);

    if home_mismatch || cache_mismatch {
        println!(
            "  Detected stale Whisper CMake cache from different source/build path; recreating build directory..."
        );
        fs::remove_dir_all(build_dir)?;
    }

    Ok(())
}

fn normalize_path(path: &Path) -> String {
    normalize_path_str(path.to_string_lossy().as_ref())
}

fn normalize_path_str(value: &str) -> String {
    let normalized = value.replace('\\', "/").trim_end_matches('/').to_string();

    if cfg!(target_os = "windows") {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}
