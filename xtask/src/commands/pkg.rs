// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use xtask::copy_dir_all_preserve_symlinks;
use xtask::{
    copy_dir_all, get_host_target_triple, ocr_sidecar_dir, project_root, qt_native_dir,
    whisper_sidecar_dir,
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

pub fn capture() -> Result<()> {
    println!("\nPackaging Capture Engine artifacts for Tauri...");

    let target_dir = project_root().join("target").join("release");
    let qt_internal_src = qt_native_dir().join("_internal");

    let app_binaries = project_root().join("apps").join("desktop").join("binaries");
    fs::create_dir_all(&app_binaries)?;

    let host_triple = get_host_target_triple()?;
    let sidecar_dir_name = format!("qt-capture-{}", host_triple);

    // Create qt-capture-{triple}/ with _internal/ nested inside
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

    // Place binary inside qt-capture-{triple}/ (no triple suffix on binary name)
    let src_binary_name = format!("capture-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let src_binary_path = target_dir.join(&src_binary_name);

    if !src_binary_path.exists() {
        anyhow::bail!("Rust binary not found: {}", src_binary_path.display());
    }

    let dst_binary_path = sidecar_dst.join(&src_binary_name);
    println!("  Copying binary to {}", dst_binary_path.display());
    fs::copy(&src_binary_path, &dst_binary_path)?;

    // Also copy to target/debug/binaries for dev
    let debug_binaries = project_root().join("target").join("debug").join("binaries");
    fs::create_dir_all(&debug_binaries)?;

    let debug_sidecar_dst = debug_binaries.join(&sidecar_dir_name);
    if debug_sidecar_dst.exists() {
        fs::remove_dir_all(&debug_sidecar_dst)?;
    }
    copy_capture_runtime_dir(&sidecar_dst, &debug_sidecar_dst)?;

    Ok(())
}

pub fn ocr() -> Result<()> {
    println!("\nPackaging OCR sidecar artifacts for Tauri...");

    let sidecar = ocr_sidecar_dir();
    let dist_dir = sidecar.join("dist");
    let app_binaries = project_root().join("apps").join("desktop").join("binaries");

    fs::create_dir_all(&app_binaries)?;

    let src_binary_name = format!("ocr-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let src_binary_path = dist_dir.join(&src_binary_name);
    let src_runtime_dir = dist_dir.join("ocr-engine");

    let host_triple = get_host_target_triple()?;
    let legacy_dst_binary_name = format!(
        "ocr-engine-{}{}",
        host_triple,
        if cfg!(windows) { ".exe" } else { "" }
    );
    let legacy_dst_binary_path = app_binaries.join(&legacy_dst_binary_name);
    let runtime_dst_dir = app_binaries.join(format!("paddle-ocr-{}", host_triple));

    // Also copy to target/debug/binaries for dev
    let debug_binaries = project_root().join("target").join("debug").join("binaries");
    fs::create_dir_all(&debug_binaries)?;
    let debug_legacy_dst_path = debug_binaries.join(&legacy_dst_binary_name);
    let debug_runtime_dst_dir = debug_binaries.join(format!("paddle-ocr-{}", host_triple));

    if src_runtime_dir.is_dir() {
        println!("  Copying OCR runtime dir to {}", runtime_dst_dir.display());
        if runtime_dst_dir.exists() {
            fs::remove_dir_all(&runtime_dst_dir)?;
        }
        copy_ocr_runtime_dir(&src_runtime_dir, &runtime_dst_dir)?;

        if legacy_dst_binary_path.exists() {
            fs::remove_file(&legacy_dst_binary_path)?;
        }

        println!(
            "  Copying OCR runtime dir to {}",
            debug_runtime_dst_dir.display()
        );
        if debug_runtime_dst_dir.exists() {
            fs::remove_dir_all(&debug_runtime_dst_dir)?;
        }
        copy_ocr_runtime_dir(&src_runtime_dir, &debug_runtime_dst_dir)?;

        if debug_legacy_dst_path.exists() {
            fs::remove_file(&debug_legacy_dst_path)?;
        }
        return Ok(());
    }

    if src_binary_path.exists() {
        println!(
            "  Copying legacy OCR binary to {}",
            legacy_dst_binary_path.display()
        );
        fs::copy(&src_binary_path, &legacy_dst_binary_path)?;

        if runtime_dst_dir.exists() {
            fs::remove_dir_all(&runtime_dst_dir)?;
        }

        println!(
            "  Copying legacy OCR binary to {}",
            debug_legacy_dst_path.display()
        );
        fs::copy(&src_binary_path, &debug_legacy_dst_path)?;
        if debug_runtime_dst_dir.exists() {
            fs::remove_dir_all(&debug_runtime_dst_dir)?;
        }
        return Ok(());
    }

    anyhow::bail!(
        "OCR artifacts not found. Expected runtime dir {} or binary {}",
        src_runtime_dir.display(),
        src_binary_path.display()
    );
}

pub fn whisper() -> Result<()> {
    println!("\nPackaging Whisper STT sidecar artifacts for Tauri...");

    let sidecar = whisper_sidecar_dir();
    let build_dir = sidecar.join("build");
    let models_dir = ensure_whisper_models(&sidecar)?;
    let app_binaries = project_root().join("apps").join("desktop").join("binaries");
    let debug_binaries = project_root().join("target").join("debug").join("binaries");

    fs::create_dir_all(&app_binaries)?;
    fs::create_dir_all(&debug_binaries)?;

    let final_src = find_whisper_binary(&build_dir)?;
    let runtime_libs = collect_whisper_runtime_libs(&build_dir)?;
    let binary_name = whisper_binary_name();

    let host_triple = get_host_target_triple()?;
    let runtime_dir_name = format!("whisper-stt-{}", host_triple);
    let legacy_dst_binary_name = format!(
        "whisper-stt-{}{}",
        host_triple,
        if cfg!(windows) { ".exe" } else { "" }
    );
    let legacy_dst_binary_path = app_binaries.join(&legacy_dst_binary_name);
    let debug_legacy_dst_path = debug_binaries.join(&legacy_dst_binary_name);

    if legacy_dst_binary_path.exists() {
        fs::remove_file(&legacy_dst_binary_path)?;
    }
    if debug_legacy_dst_path.exists() {
        fs::remove_file(&debug_legacy_dst_path)?;
    }

    let sidecar_dst = app_binaries.join(&runtime_dir_name);
    if sidecar_dst.exists() {
        fs::remove_dir_all(&sidecar_dst)?;
    }
    fs::create_dir_all(&sidecar_dst)?;

    let dst_binary_path = sidecar_dst.join(binary_name);
    println!("  Copying binary to {}", dst_binary_path.display());
    fs::copy(&final_src, &dst_binary_path)?;

    let internal_dst = sidecar_dst.join("_internal");
    fs::create_dir_all(&internal_dst)?;

    for lib in runtime_libs {
        let Some(name) = lib.file_name() else {
            continue;
        };
        let dst = internal_dst.join(name);
        println!("  Copying runtime lib to {}", dst.display());
        fs::copy(&lib, &dst)?;
    }

    let models_dst = internal_dst.join("models");
    println!("  Copying models to {}", models_dst.display());
    if models_dst.exists() {
        fs::remove_dir_all(&models_dst)?;
    }
    copy_dir_all(&models_dir, &models_dst)?;

    let debug_sidecar_dst = debug_binaries.join(&runtime_dir_name);
    if debug_sidecar_dst.exists() {
        fs::remove_dir_all(&debug_sidecar_dst)?;
    }
    copy_dir_all(&sidecar_dst, &debug_sidecar_dst)?;

    Ok(())
}

fn whisper_binary_name() -> &'static str {
    if cfg!(windows) {
        "whisper-stt.exe"
    } else {
        "whisper-stt"
    }
}

fn ensure_whisper_models(sidecar: &Path) -> Result<PathBuf> {
    let models_dir = sidecar.join("models");
    let required_model = models_dir.join("ggml-tiny.en.bin");
    if !required_model.exists() {
        anyhow::bail!(
            "Whisper models are missing. Expected at least {}.\nRun: python sidecars/whisper-stt/download_models.py",
            required_model.display()
        );
    }
    Ok(models_dir)
}

fn find_whisper_binary(build_dir: &Path) -> Result<PathBuf> {
    let name = whisper_binary_name();
    let mut candidates = vec![
        build_dir.join("Release").join(name),
        build_dir.join(name),
        build_dir.join("bin").join("Release").join(name),
        build_dir.join("bin").join(name),
    ];
    candidates.dedup();

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Whisper binary not found. Expected one of:\n  - {}",
        [
            build_dir.join("Release").join(name).display().to_string(),
            build_dir.join(name).display().to_string(),
            build_dir
                .join("bin")
                .join("Release")
                .join(name)
                .display()
                .to_string(),
            build_dir.join("bin").join(name).display().to_string()
        ]
        .join("\n  - ")
    )
}

fn collect_whisper_runtime_libs(build_dir: &Path) -> Result<Vec<PathBuf>> {
    let candidates = [
        build_dir.join("bin").join("Release"),
        build_dir.join("bin"),
        build_dir.join("Release"),
        build_dir.join("_deps").join("whisper_cpp-build").join("ggml").join("src"),
        build_dir.join("_deps").join("whisper_cpp-build").join("src"),
    ];

    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for dir in candidates {
        if !dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let lower = file_name.to_ascii_lowercase();
            if !(lower.contains("whisper") || lower.contains("ggml")) {
                continue;
            }
            if !is_whisper_runtime_lib_name(&lower) {
                continue;
            }
            if seen.insert(lower) {
                out.push(path);
            }
        }
    }

    if out.is_empty() {
        anyhow::bail!(
            "Whisper runtime libraries were not found in build outputs. Expected whisper/ggml shared libraries under {}/bin or {}/Release",
            build_dir.display(),
            build_dir.display()
        );
    }

    Ok(out)
}

fn is_whisper_runtime_lib_name(name: &str) -> bool {
    #[cfg(windows)]
    {
        return name.ends_with(".dll");
    }

    #[cfg(target_os = "macos")]
    {
        return name.ends_with(".dylib");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return name.ends_with(".so") || name.contains(".so.");
    }

    #[allow(unreachable_code)]
    false
}
