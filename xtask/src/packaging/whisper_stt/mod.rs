// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use xtask::{copy_dir_all, get_host_target_triple, project_root, stt_sidecar_dir};

pub fn stt() -> Result<()> {
    println!("\nPackaging Whisper STT sidecar artifacts for distribution...");

    let sidecar = stt_sidecar_dir();
    let build_dir = sidecar.join("build");
    let models_dir = ensure_stt_models(&sidecar)?;
    let pkg_binaries = project_root().join("packaging").join("binaries");
    fs::create_dir_all(&pkg_binaries)?;

    let final_src = find_stt_binary(&build_dir)?;
    let runtime_libs = collect_stt_runtime_libs(&build_dir)?;
    let binary_name = stt_binary_name();

    let host_triple = get_host_target_triple()?;
    let runtime_dir_name = format!("whisper-stt-{}", host_triple);
    let sidecar_dst = pkg_binaries.join(&runtime_dir_name);
    
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

    Ok(())
}

fn stt_binary_name() -> &'static str {
    if cfg!(windows) {
        "squigit-stt.exe"
    } else {
        "squigit-stt"
    }
}

fn ensure_stt_models(sidecar: &Path) -> Result<PathBuf> {
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

fn find_stt_binary(build_dir: &Path) -> Result<PathBuf> {
    let name = stt_binary_name();
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

fn collect_stt_runtime_libs(build_dir: &Path) -> Result<Vec<PathBuf>> {
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
            if !is_stt_runtime_lib_name(&lower) {
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

fn is_stt_runtime_lib_name(name: &str) -> bool {
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
