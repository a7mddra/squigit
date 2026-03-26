// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::fs;
use std::path::Path;
#[cfg(not(windows))]
use xtask::copy_dir_all_preserve_symlinks;
use xtask::{
    get_host_target_triple, ocr_sidecar_dir, project_root,
};

fn copy_ocr_runtime_dir(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        copy_dir_all(src, dst)?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        copy_dir_all_preserve_symlinks(src, dst)?;
        verify_symlink_integrity(src, dst)?;
        Ok(())
    }
}

#[cfg(not(windows))]
fn count_symlinks_recursive(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            count += 1;
            continue;
        }
        if file_type.is_dir() {
            count += count_symlinks_recursive(&entry.path())?;
        }
    }
    Ok(count)
}

#[cfg(not(windows))]
fn verify_symlink_integrity(src: &Path, dst: &Path) -> Result<()> {
    let src_count = count_symlinks_recursive(src)?;
    let dst_count = count_symlinks_recursive(dst)?;
    if src_count != dst_count {
        anyhow::bail!(
            "OCR runtime symlink integrity failed: src={} dst={} ({} -> {})",
            src_count,
            dst_count,
            src.display(),
            dst.display()
        );
    }
    Ok(())
}

pub fn ocr() -> Result<()> {
    println!("\nPackaging OCR sidecar artifacts for distribution...");

    let sidecar = ocr_sidecar_dir();
    let dist_dir = sidecar.join("dist");
    
    // NEW DESTINATION
    let pkg_binaries = project_root().join("packaging").join("binaries");
    fs::create_dir_all(&pkg_binaries)?;

    let src_binary_name = format!("squigit-ocr{}", if cfg!(windows) { ".exe" } else { "" });
    let src_binary_path = dist_dir.join(&src_binary_name);
    let src_runtime_dir = dist_dir.join("squigit-ocr");

    let host_triple = get_host_target_triple()?;
    let runtime_dst_dir = pkg_binaries.join(format!("paddle-ocr-{}", host_triple));

    if src_runtime_dir.is_dir() {
        println!("  Copying OCR runtime dir to {}", runtime_dst_dir.display());
        if runtime_dst_dir.exists() {
            fs::remove_dir_all(&runtime_dst_dir)?;
        }
        copy_ocr_runtime_dir(&src_runtime_dir, &runtime_dst_dir)?;
        return Ok(());
    }

    if src_binary_path.exists() {
        if runtime_dst_dir.exists() {
            fs::remove_dir_all(&runtime_dst_dir)?;
        }
        fs::create_dir_all(&runtime_dst_dir)?;
        let legacy_dst_binary_path = runtime_dst_dir.join(&src_binary_name);
        println!(
            "  Copying legacy OCR binary to {}",
            legacy_dst_binary_path.display()
        );
        fs::copy(&src_binary_path, &legacy_dst_binary_path)?;
        return Ok(());
    }

    anyhow::bail!(
        "OCR artifacts not found. Expected runtime dir {} or binary {}",
        src_runtime_dir.display(),
        src_binary_path.display()
    );
}
