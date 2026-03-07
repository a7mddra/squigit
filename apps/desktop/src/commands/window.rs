// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use tauri::window::Color;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    println!("Opening URL: {}", url);
    tauri::async_runtime::spawn_blocking(move || crate::utils::open_url(&url))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_background_color(app: AppHandle, color: String) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let color = color.trim_start_matches('#');
    let (r, g, b) = if color.len() == 6 {
        let r = u8::from_str_radix(&color[0..2], 16).map_err(|e| e.to_string())?;
        let g = u8::from_str_radix(&color[2..4], 16).map_err(|e| e.to_string())?;
        let b = u8::from_str_radix(&color[4..6], 16).map_err(|e| e.to_string())?;
        (r, g, b)
    } else {
        return Err("Invalid color format. Use #RRGGBB".to_string());
    };

    let _ = window.set_background_color(Some(Color(r, g, b, 255)));
    Ok(())
}

#[tauri::command]
pub fn minimize_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn maximize_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn close_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reload_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window
        .eval("window.location.reload()")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_always_on_top(app: AppHandle, state: bool) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.set_always_on_top(state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    crate::services::tray::show_window(&app);
    Ok(())
}

#[tauri::command]
pub fn get_always_on_top(app: AppHandle) -> Result<bool, String> {
    let _window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    Ok(false)
}
