// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

//! Windows Qt deployment.
//!
//! Builds Qt project with CMake and uses windeployqt for bundling.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build(native_dir: &Path) -> Result<()> {
    let build_dir = native_dir.join("build");

    let qt_path = find_qt_path()?;
    println!("  Qt Path: {}", qt_path);

    println!("  Configuring CMake...");
    let configured = if has_msvc_compiler_in_path() {
        println!("  Using Ninja generator with MSVC from PATH.");
        run_cmake_configure(
            native_dir,
            &build_dir,
            &qt_path,
            "Ninja",
            &["-DCMAKE_C_COMPILER=cl.exe", "-DCMAKE_CXX_COMPILER=cl.exe"],
        )?
    } else {
        let Some(generator) = find_visual_studio_generator() else {
            anyhow::bail!(
                "cl.exe is not on PATH and no compatible Visual Studio C++ toolchain was detected. Install Visual Studio Build Tools (Desktop development with C++) or run from a Developer PowerShell."
            );
        };
        println!(
            "  cl.exe is not on PATH; using CMake generator '{}'.",
            generator
        );
        run_cmake_configure(native_dir, &build_dir, &qt_path, generator, &["-A", "x64"])?
    };

    if !configured {
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

    let mut args = vec![
        "-S".to_string(),
        native_dir.to_string_lossy().to_string(),
        "-B".to_string(),
        build_dir.to_string_lossy().to_string(),
        "-G".to_string(),
        generator.to_string(),
        "-DCMAKE_BUILD_TYPE=Release".to_string(),
        format!("-DCMAKE_PREFIX_PATH={}", qt_path),
    ];
    args.extend(extra_args.iter().map(|arg| arg.to_string()));

    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let status = Command::new("cmake")
        .args(&arg_refs)
        .status()
        .context("Failed to run cmake configure")?;

    Ok(status.success())
}

fn has_msvc_compiler_in_path() -> bool {
    which::which("cl.exe").is_ok() || which::which("cl").is_ok()
}

fn find_visual_studio_generator() -> Option<&'static str> {
    let candidates = [
        ("Visual Studio 17 2022", "[17.0,18.0)"),
        ("Visual Studio 16 2019", "[16.0,17.0)"),
    ];

    for (generator, version_range) in candidates {
        if has_visual_studio_msvc(version_range) {
            return Some(generator);
        }
    }

    None
}

fn has_visual_studio_msvc(version_range: &str) -> bool {
    let vswhere =
        Path::new(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe");
    if !vswhere.exists() {
        return false;
    }

    let output = match Command::new(vswhere)
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
    {
        Ok(output) => output,
        Err(_) => return false,
    };

    output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
}

pub fn deploy(native_dir: &Path) -> Result<()> {
    let build_dir = native_dir.join("build");
    let dist_dir = native_dir.join("_internal");

    let qt_path = find_qt_path()?;

    println!("  Running windeployqt...");
    create_distribution(native_dir, &build_dir, &dist_dir, &qt_path)?;

    Ok(())
}

fn find_qt_path() -> Result<String> {
    let mut candidates = Vec::new();

    if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
        candidates.push(normalize_qt_prefix(PathBuf::from(qt6_dir)));
    }

    if let Ok(qtdir) = std::env::var("QTDIR") {
        candidates.push(PathBuf::from(qtdir));
    }

    if let Ok(prefix_path) = std::env::var("CMAKE_PREFIX_PATH") {
        for path in std::env::split_paths(&prefix_path) {
            candidates.push(normalize_qt_prefix(path));
        }
    }

    if let Ok(output) = Command::new("qmake")
        .arg("-query")
        .arg("QT_INSTALL_PREFIX")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                candidates.push(PathBuf::from(path));
            }
        }
    }

    for candidate in [
        r"C:\Qt\6.8.0\msvc2022_64",
        r"C:\Qt\6.8.0\msvc2019_64",
        r"C:\Qt\6.7.0\msvc2022_64",
        r"C:\Qt\6.7.0\msvc2019_64",
        r"C:\Qt\6.6.0\msvc2022_64",
        r"C:\Qt\6.6.0\msvc2019_64",
    ] {
        candidates.push(PathBuf::from(candidate));
    }

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
    let qt6_config = path.join("Qt6Config.cmake");
    let is_qt6_cmake_dir = path
        .file_name()
        .map(|name| name.to_string_lossy().eq_ignore_ascii_case("Qt6"))
        .unwrap_or(false)
        && qt6_config.exists();

    if is_qt6_cmake_dir {
        if let Some(prefix) = path.parent().and_then(Path::parent).and_then(Path::parent) {
            return prefix.to_path_buf();
        }
    }

    path
}

fn dedupe_existing_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        if out.iter().any(|existing| existing == &path) {
            continue;
        }
        out.push(path);
    }
    out
}

fn qt_has_required_modules(prefix: &Path) -> bool {
    let required = [
        prefix.join(r"lib\cmake\Qt6\Qt6Config.cmake"),
        prefix.join(r"lib\cmake\Qt6Quick\Qt6QuickConfig.cmake"),
        prefix.join(r"lib\cmake\Qt6Qml\Qt6QmlConfig.cmake"),
        prefix.join(r"qml\Qt5Compat\GraphicalEffects\qmldir"),
    ];

    required.iter().all(|p| p.exists())
}

fn create_distribution(
    native_dir: &Path,
    build_dir: &Path,
    dist_dir: &Path,
    qt_path: &str,
) -> Result<()> {
    if dist_dir.exists() {
        fs::remove_dir_all(dist_dir)?;
    }
    fs::create_dir_all(dist_dir)?;

    let exe_name = "capture.exe";
    let exe_src = build_dir.join(exe_name);
    let exe_dst = dist_dir.join(exe_name);

    if !exe_src.exists() {
        let release_exe = build_dir.join("Release").join(exe_name);
        if release_exe.exists() {
            fs::copy(&release_exe, &exe_dst)?;
        } else {
            anyhow::bail!("Built exe not found: {}", exe_src.display());
        }
    } else {
        fs::copy(&exe_src, &exe_dst)?;
    }

    let windeployqt = Path::new(qt_path).join("bin").join("windeployqt.exe");

    if windeployqt.exists() {
        let qml_dir = native_dir.join("qml");
        let qml_dir_arg = qml_dir.to_string_lossy().to_string();
        let status = Command::new(&windeployqt)
            .current_dir(dist_dir)
            .args([
                exe_name,
                "--release",
                "--qmldir",
                &qml_dir_arg,
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
    } else {
        anyhow::bail!("windeployqt not found at {}", windeployqt.display());
    }

    ensure_qt5compat_graphicaleffects(dist_dir, qt_path)?;
    bundle_vc_runtime(dist_dir)?;

    Ok(())
}

fn ensure_qt5compat_graphicaleffects(dist_dir: &Path, qt_path: &str) -> Result<()> {
    let qt_qml_root = Path::new(qt_path).join("qml");
    let src_qt5compat = qt_qml_root.join("Qt5Compat");
    let src_graphicaleffects_qmldir = src_qt5compat.join("GraphicalEffects").join("qmldir");

    if !src_graphicaleffects_qmldir.exists() {
        anyhow::bail!(
            "Missing Qt module 'Qt5Compat.GraphicalEffects' under {}.\n\
Install Qt5Compat for this Qt kit (Qt Maintenance Tool or aqt module 'qt5compat').",
            qt_qml_root.display()
        );
    }

    let dst_qt5compat = dist_dir.join("qml").join("Qt5Compat");
    copy_dir_recursive(&src_qt5compat, &dst_qt5compat)?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn bundle_vc_runtime(dist_dir: &Path) -> Result<()> {
    let system32 = std::env::var("SystemRoot")
        .map(|r| Path::new(&r).join("System32"))
        .unwrap_or_else(|_| Path::new(r"C:\Windows\System32").to_path_buf());

    let runtime_dlls = [
        "vcruntime140.dll",
        "vcruntime140_1.dll",
        "msvcp140.dll",
        "msvcp140_1.dll",
        "ucrtbase.dll",
    ];

    for dll in runtime_dlls {
        let src = system32.join(dll);
        if src.exists() {
            let dst = dist_dir.join(dll);
            if !dst.exists() {
                fs::copy(&src, &dst)?;
            }
        }
    }

    Ok(())
}
