// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use xtask::copy_dir_all_preserve_symlinks;
use xtask::{
    get_host_target_triple, project_root, qt_native_dir,
};

fn copy_capture_runtime_dir(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        copy_dir_all(src, dst)?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        copy_dir_all_preserve_symlinks(src, dst)?;
        Ok(())
    }
}

pub fn capture() -> Result<()> {
    println!("\nPackaging Capture Engine artifacts for Tauri...");

    let target_dir = project_root().join("target").join("release");
    let qt_internal_src = qt_native_dir().join("_internal");

    let app_binaries = project_root().join("apps").join("desktop").join("binaries");
    fs::create_dir_all(&app_binaries)?;

    let host_triple = get_host_target_triple()?;
    let sidecar_dir_name = format!("qt-capture-{}", host_triple);

    
    let sidecar_dst = app_binaries.join(&sidecar_dir_name);
    if sidecar_dst.exists() {
        fs::remove_dir_all(&sidecar_dst)?;
    }
    fs::create_dir_all(&sidecar_dst)?;

    let internal_dst = sidecar_dst.join("_internal");

    if !qt_internal_src.exists() {
        anyhow::bail!("Qt runtime not found at {}", qt_internal_src.display());
    }

    println!("  Moving _internal to {}", internal_dst.display());
    if fs::rename(&qt_internal_src, &internal_dst).is_err() {
        copy_capture_runtime_dir(&qt_internal_src, &internal_dst)?;
        fs::remove_dir_all(&qt_internal_src)?;
    }

    
    let src_binary_name = format!("capture-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let src_binary_path = target_dir.join(&src_binary_name);

    if !src_binary_path.exists() {
        anyhow::bail!("Rust binary not found: {}", src_binary_path.display());
    }

    let dst_binary_path = sidecar_dst.join(&src_binary_name);
    println!("  Copying binary to {}", dst_binary_path.display());
    fs::copy(&src_binary_path, &dst_binary_path)?;

    
    let debug_binaries = project_root().join("target").join("debug").join("binaries");
    fs::create_dir_all(&debug_binaries)?;

    let debug_sidecar_dst = debug_binaries.join(&sidecar_dir_name);
    if debug_sidecar_dst.exists() {
        fs::remove_dir_all(&debug_sidecar_dst)?;
    }
    copy_capture_runtime_dir(&sidecar_dst, &debug_sidecar_dst)?;

    Ok(())
}
