// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn_capture(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || match run_capture(&handle, false) {
        Ok(result) => {
            if let Some(window) = handle.get_webview_window("main") {
                let was_hidden =
                    !window.is_visible().unwrap_or(true) || window.is_minimized().unwrap_or(false);

                if was_hidden {
                    if let Some(geo) = result.display_geo {
                        let win_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
                            width: 1030,
                            height: 690,
                        });
                        let center_x = geo.x + (geo.w as i32 - win_size.width as i32) / 2;
                        let center_y = geo.y + (geo.h as i32 - win_size.height as i32) / 2;
                        let _ =
                            window.set_position(tauri::PhysicalPosition::new(center_x, center_y));
                    }
                    let _ = window.unminimize();
                    let _ = window.show();
                }
                let _ = window.set_focus();
            }

            let payload = serde_json::json!({
                "chatId": result.chat_id,
                "imageHash": result.image_hash,
            });
            let _ = handle.emit("capture-complete", payload);
        }
        Err(e) => {
            let _ = handle.emit("capture-failed", serde_json::json!({ "reason": e }));
        }
    });
}

pub fn spawn_capture_to_input(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || match run_capture(&handle, true) {
        Ok(result) => {
            if let Some(window) = handle.get_webview_window("main") {
                let was_hidden =
                    !window.is_visible().unwrap_or(true) || window.is_minimized().unwrap_or(false);

                if was_hidden {
                    if let Some(geo) = result.display_geo {
                        let win_size = window.outer_size().unwrap_or(tauri::PhysicalSize {
                            width: 1030,
                            height: 690,
                        });
                        let center_x = geo.x + (geo.w as i32 - win_size.width as i32) / 2;
                        let center_y = geo.y + (geo.h as i32 - win_size.height as i32) / 2;
                        let _ =
                            window.set_position(tauri::PhysicalPosition::new(center_x, center_y));
                    }
                    let _ = window.unminimize();
                    let _ = window.show();
                }
                let _ = window.set_focus();
            }

            if let Some(temp_path) = result.temp_path {
                let _ = handle.emit(
                    "capture-to-input",
                    serde_json::json!({ "tempPath": temp_path }),
                );
            }
        }
        Err(e) => {
            let _ = handle.emit("capture-failed", serde_json::json!({ "reason": e }));
        }
    });
}

struct CaptureResult {
    chat_id: String,
    image_hash: String,
    temp_path: Option<String>,
    display_geo: Option<DisplayGeo>,
}

#[derive(Debug, Clone)]
struct DisplayGeo {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

fn run_capture(app: &AppHandle, input_only: bool) -> Result<CaptureResult, String> {
    let sidecar_path = resolve_sidecar_path(app)?;

    let mut args = Vec::new();
    if input_only {
        args.push("--input-only".to_string());
    }
    let mut is_freeshape = false;

    if let Ok(config_dir) = app.path().app_config_dir() {
        let prefs_path = config_dir.join("preferences.json");
        if let Ok(contents) = std::fs::read_to_string(&prefs_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(capture_type) = json.get("captureType").and_then(|v| v.as_str()) {
                    if capture_type == "squiggle" {
                        is_freeshape = true;
                    }
                }
            }
        }
    }

    if is_freeshape {
        args.push("-f".to_string());
    } else {
        args.push("-r".to_string());
    }

    let mut child = Command::new(&sidecar_path)
        .args(&args)
        .env(
            "GIO_LAUNCHED_DESKTOP_APP_ID",
            crate::constants::APP_NAME.to_lowercase(),
        )
        .env(
            "G_APPLICATION_ID",
            crate::constants::APP_NAME.to_lowercase(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn capture sidecar: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to get capture sidecar stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut chat_id: Option<String> = None;
    let mut image_hash: Option<String> = None;
    let mut temp_path: Option<String> = None;
    let mut display_geo: Option<DisplayGeo> = None;

    for line in reader.lines() {
        match line {
            Ok(msg) => {
                let trimmed = msg.trim();
                if let Some(id) = trimmed.strip_prefix("CHAT_ID:") {
                    chat_id = Some(id.to_string());
                } else if let Some(hash) = trimmed.strip_prefix("IMAGE_HASH:") {
                    image_hash = Some(hash.to_string());
                } else if let Some(path) = trimmed.strip_prefix("CAS_PATH:") {
                    temp_path = Some(path.to_string());
                } else if let Some(geo_str) = trimmed.strip_prefix("DISPLAY_GEO:") {
                    display_geo = parse_display_geo(geo_str);
                } else if trimmed == "CAPTURE_DENIED" {
                    return Err("User denied screen capture permission.".to_string());
                }
            }
            Err(_) => break,
        }
    }

    let _ = child.wait();

    if input_only {
        let path =
            temp_path.ok_or_else(|| "Capture sidecar did not return CAS_PATH".to_string())?;
        Ok(CaptureResult {
            chat_id: String::new(),
            image_hash: String::new(),
            temp_path: Some(path),
            display_geo,
        })
    } else {
        let chat_id =
            chat_id.ok_or_else(|| "Capture sidecar did not return CHAT_ID".to_string())?;
        let image_hash = image_hash.unwrap_or_default();

        Ok(CaptureResult {
            chat_id,
            image_hash,
            temp_path: None,
            display_geo,
        })
    }
}

fn parse_display_geo(s: &str) -> Option<DisplayGeo> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 4 {
        Some(DisplayGeo {
            x: parts[0].parse().ok()?,
            y: parts[1].parse().ok()?,
            w: parts[2].parse().ok()?,
            h: parts[3].parse().ok()?,
        })
    } else {
        None
    }
}

fn resolve_sidecar_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let binary_name = format!("capture-engine{}", if cfg!(windows) { ".exe" } else { "" });
    let sidecar_dir_name = format!("qt-capture-{}", get_capture_target_triple());

    if let Ok(resource_dir) = app.path().resource_dir() {
        let prod_path = resource_dir
            .join("binaries")
            .join(&sidecar_dir_name)
            .join(&binary_name);
        if prod_path.exists() {
            return Ok(prod_path);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let dev_path = root
                .join("sidecars")
                .join("qt-capture")
                .join("target")
                .join("release")
                .join("capture-engine");
            if dev_path.exists() {
                return Ok(dev_path);
            }

            let debug_path = root
                .join("sidecars")
                .join("qt-capture")
                .join("target")
                .join("debug")
                .join("capture-engine");
            if debug_path.exists() {
                return Ok(debug_path);
            }
        }
    }

    Err(format!(
        "Capture sidecar not found. Searched production and dev locations for: {}/{}",
        sidecar_dir_name, binary_name
    ))
}

fn get_capture_target_triple() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(target_os = "macos")]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(target_os = "linux")]
    {
        "x86_64-unknown-linux-gnu"
    }
}
