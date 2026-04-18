// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! System level commands for orchestrating sidecars and OS environment checks

use crate::services::ocr::DesktopOcrService;
use tauri::Manager;

#[tauri::command]
pub async fn run_sidecar_version(
    app: tauri::AppHandle,
    ocr: tauri::State<'_, DesktopOcrService>,
    command: String,
) -> Result<String, String> {
    if command == "squigit-ocr --version" {
        let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
        let (sidecar_path, _) = ocr.resolve_sidecar_path(&resource_dir);
        return ocr.read_sidecar_version(&sidecar_path);
    }

    if command == "squigit-stt --version" {
        let (sidecar_path, _) = crate::commands::speech::resolve_sidecar_path(&app)?;
        let output = std::process::Command::new(sidecar_path)
            .arg("--version")
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
        return Err("Sidecar command failed".to_string());
    }

    Err("Unknown or unsupported sidecar command".to_string())
}

#[tauri::command]
pub async fn get_linux_package_manager() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let debian = std::path::Path::new("/etc/debian_version").exists()
            || std::path::Path::new("/usr/bin/dpkg").exists()
            || std::path::Path::new("/bin/dpkg").exists();
        if debian {
            return Ok("debian".to_string());
        }
        let rpm = std::path::Path::new("/etc/redhat-release").exists()
            || std::path::Path::new("/etc/fedora-release").exists()
            || std::path::Path::new("/usr/bin/rpm").exists()
            || std::path::Path::new("/bin/rpm").exists();
        if rpm {
            return Ok("rpm".to_string());
        }
    }
    Ok("unknown".to_string())
}
