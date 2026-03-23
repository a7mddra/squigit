// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn capture_sidecar_dir() -> PathBuf {
    project_root().join("sidecars").join("qt-capture")
}

pub fn qt_native_dir() -> PathBuf {
    capture_sidecar_dir().join("native")
}

pub fn ocr_sidecar_dir() -> PathBuf {
    project_root().join("sidecars").join("paddle-ocr")
}

pub fn whisper_sidecar_dir() -> PathBuf {
    project_root().join("sidecars").join("whisper-stt")
}

pub fn venv_python() -> PathBuf {
    let sidecar = ocr_sidecar_dir();
    if cfg!(windows) {
        sidecar.join("venv").join("Scripts").join("python.exe")
    } else {
        sidecar.join("venv").join("bin").join("python")
    }
}

pub fn ui_dir() -> PathBuf {
    project_root().join("apps").join("desktop").join("renderer")
}

pub fn tauri_dir() -> PathBuf {
    project_root().join("apps").join("desktop")
}

fn resolve_command_path(cmd: &str) -> PathBuf {
    let cmd_path = Path::new(cmd);
    if cmd_path.components().count() > 1 || cmd_path.extension().is_some() {
        return cmd_path.to_path_buf();
    }

    if let Ok(path) = which::which(cmd) {
        return path;
    }

    #[cfg(windows)]
    {
        for ext in ["cmd", "exe", "bat"] {
            let candidate = format!("{cmd}.{ext}");
            if let Ok(path) = which::which(&candidate) {
                return path;
            }
        }

        if let Some(path) = windows_node_tool_fallback(cmd) {
            return path;
        }
    }

    cmd_path.to_path_buf()
}

#[cfg(windows)]
fn windows_node_tool_fallback(cmd: &str) -> Option<PathBuf> {
    let names: &[&str] = match cmd {
        "npm" => &["npm.cmd", "npm.exe"],
        "npx" => &["npx.cmd", "npx.exe"],
        "node" => &["node.exe"],
        _ => return None,
    };

    let mut roots = Vec::new();
    if let Some(program_files) = env::var_os("ProgramFiles") {
        roots.push(PathBuf::from(program_files).join("nodejs"));
    }
    if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files_x86).join("nodejs"));
    }
    if let Some(local_app_data) = env::var_os("LocalAppData") {
        roots.push(
            PathBuf::from(local_app_data)
                .join("Programs")
                .join("nodejs"),
        );
    }

    for root in roots {
        for name in names {
            let candidate = root.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn run_cmd(cmd: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let command_path = resolve_command_path(cmd);
    println!("  $ {} {}", command_path.display(), args.join(" "));
    let status = Command::new(&command_path)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to run: {} {:?}", command_path.display(), args))?;

    if !status.success() {
        anyhow::bail!("Command failed with exit code: {:?}", status.code());
    }
    Ok(())
}

pub fn run_cmd_with_node_bin(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    node_bin_dir: &Path,
) -> Result<()> {
    run_cmd_with_node_bin_and_env(cmd, args, cwd, node_bin_dir, &[])
}

pub fn run_cmd_with_node_bin_and_env(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    node_bin_dir: &Path,
    env_vars: &[(String, String)],
) -> Result<()> {
    let path_var = env::var("PATH").unwrap_or_default();
    #[cfg(windows)]
    let new_path = format!("{};{}", node_bin_dir.display(), path_var);
    #[cfg(not(windows))]
    let new_path = format!("{}:{}", node_bin_dir.display(), path_var);

    let path_env_key = if cfg!(windows) { "Path" } else { "PATH" };

    let command_path = if cfg!(windows) {
        let cmd_shim = node_bin_dir.join(format!("{cmd}.cmd"));
        let exe_shim = node_bin_dir.join(format!("{cmd}.exe"));
        if cmd_shim.exists() {
            cmd_shim
        } else if exe_shim.exists() {
            exe_shim
        } else {
            resolve_command_path(cmd)
        }
    } else {
        let shim = node_bin_dir.join(cmd);
        if shim.exists() {
            shim
        } else {
            resolve_command_path(cmd)
        }
    };
    println!("  $ {} {}", command_path.display(), args.join(" "));

    let mut command = Command::new(&command_path);
    command
        .args(args)
        .current_dir(cwd)
        .env(path_env_key, new_path);

    for (key, value) in env_vars {
        command.env(key, value);
    }

    let status = command
        .status()
        .with_context(|| format!("Failed to run: {} {:?}", command_path.display(), args))?;

    if !status.success() {
        anyhow::bail!("Command failed with exit code: {:?}", status.code());
    }
    Ok(())
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn remove_existing_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn copy_dir_all_preserve_symlinks(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all_preserve_symlinks(&src_path, &dst_path)?;
            continue;
        }

        if ty.is_symlink() {
            remove_existing_path(&dst_path)?;
            #[cfg(unix)]
            {
                let link_target = fs::read_link(&src_path)?;
                unix_fs::symlink(&link_target, &dst_path)?;
            }
            #[cfg(windows)]
            {
                fs::copy(&src_path, &dst_path)?;
            }
            continue;
        }

        fs::copy(&src_path, &dst_path)?;
    }
    Ok(())
}

pub fn get_host_target_triple() -> Result<String> {
    let output = Command::new("rustc").arg("-vV").output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("host: ") {
            return Ok(line.trim_start_matches("host: ").trim().to_string());
        }
    }
    Ok("unknown-target".to_string())
}
