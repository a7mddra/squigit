// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Returns the initial background color based on the saved theme preference,
/// falling back to system theme detection.
fn initial_bg_color(app: &AppHandle) -> tauri::window::Color {
    let is_light = match resolve_saved_theme(app) {
        Some(theme) => theme == "light",
        None => crate::services::theme::get_system_theme() == "light",
    };

    if is_light {
        tauri::window::Color(255, 255, 255, 255) // --c-raw-013 light: #ffffff
    } else {
        tauri::window::Color(15, 15, 15, 255) // --c-raw-013 dark: #0f0f0f
    }
}

/// Reads the saved theme preference from preferences.json.
/// Returns None if the theme is "system" or if reading fails.
fn resolve_saved_theme(app: &AppHandle) -> Option<String> {
    let config_dir = crate::utils::get_app_config_dir(app);
    let prefs_file = config_dir.join("preferences.json");
    let content = std::fs::read_to_string(prefs_file).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let theme = json.get("theme")?.as_str()?;
    match theme {
        "light" | "dark" => Some(theme.to_string()),
        _ => None, // "system" → fall back to system detection
    }
}

pub fn calculate_dynamic_window(
    app: &AppHandle,
    base_w: f64,
    base_h: f64,
) -> Result<(f64, f64, f64, f64), String> {
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No monitor found")?;
    Ok(center_in_monitor_work_area(&monitor, base_w, base_h))
}

pub fn center_on_cursor_monitor(app: &AppHandle, base_w: f64, base_h: f64) -> (f64, f64, f64, f64) {
    if let Ok(cursor) = app.cursor_position() {
        if let Ok(monitors) = app.available_monitors() {
            for monitor in monitors {
                let (mx, my, mw, mh) = monitor_bounds(&monitor);
                if point_in_rect(cursor.x, cursor.y, mx, my, mw, mh) {
                    return center_in_monitor_work_area(&monitor, base_w, base_h);
                }
            }
        }
    }
    calculate_dynamic_window(app, base_w, base_h).unwrap_or((100.0, 100.0, base_w, base_h))
}

fn point_in_rect(px: f64, py: f64, rx: f64, ry: f64, rw: f64, rh: f64) -> bool {
    px >= rx && px < rx + rw && py >= ry && py < ry + rh
}

fn monitor_bounds(monitor: &tauri::Monitor) -> (f64, f64, f64, f64) {
    let pos = monitor.position();
    let size = monitor.size();
    (
        pos.x as f64,
        pos.y as f64,
        size.width as f64,
        size.height as f64,
    )
}

fn monitor_work_area(monitor: &tauri::Monitor) -> (f64, f64, f64, f64) {
    let work = monitor.work_area();
    let work_w = work.size.width as f64;
    let work_h = work.size.height as f64;

    if work_w > 0.0 && work_h > 0.0 {
        (
            work.position.x as f64,
            work.position.y as f64,
            work_w,
            work_h,
        )
    } else {
        monitor_bounds(monitor)
    }
}

fn center_in_monitor_work_area(
    monitor: &tauri::Monitor,
    base_w: f64,
    base_h: f64,
) -> (f64, f64, f64, f64) {
    let (area_x, area_y, area_w, area_h) = monitor_work_area(monitor);
    let (win_w, win_h) = compute_aspect_locked_size(area_w, area_h, base_w, base_h);

    let x = area_x + (area_w - win_w) / 2.0;
    let y = area_y + (area_h - win_h) / 2.0;
    (x.floor(), y.floor(), win_w, win_h)
}

fn compute_aspect_locked_size(area_w: f64, area_h: f64, base_w: f64, base_h: f64) -> (f64, f64) {
    if area_w <= 1.0 || area_h <= 1.0 || base_w <= 1.0 || base_h <= 1.0 {
        return (base_w.max(1.0).floor(), base_h.max(1.0).floor());
    }

    // Keep original relative sizing intent against a 1366x768 reference monitor.
    let requested_w = (base_w / 1366.0) * area_w;
    let requested_h = (base_h / 768.0) * area_h;

    // Use the dominant requested axis, then clamp so the locked aspect ratio fits the work area.
    let requested_scale = (requested_w / base_w).max(requested_h / base_h);
    let max_fit_scale = (area_w / base_w).min(area_h / base_h);
    let scale = requested_scale.min(max_fit_scale).max(0.0);

    let win_w = (base_w * scale).floor().clamp(1.0, area_w.floor());
    let win_h = (base_h * scale).floor().clamp(1.0, area_h.floor());
    (win_w, win_h)
}

pub fn spawn_app_window(
    app: &AppHandle,
    label: &str,
    url: &str,
    base_w: f64,
    base_h: f64,
    title: &str,
    visible: bool,
) -> Result<(), String> {
    if app.get_webview_window(label).is_some() {
        return Ok(());
    }

    let (x, y, w, h) = center_on_cursor_monitor(app, base_w, base_h);

    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .position(x, y)
        .inner_size(w, h)
        .visible(visible)
        .resizable(true)
        .decorations(false)
        .background_color(initial_bg_color(app))
        .build()
        .map_err(|e| e.to_string())?;

    let window_clone = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            let _ = window_clone.hide();
            api.prevent_close();
        }
        WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
            if let Some(first_path) = paths.first() {
                let path_str = first_path.to_string_lossy().to_string();
                let state = window_clone.state::<crate::state::AppState>();

                match crate::services::image::process_and_store_image(path_str.clone(), &state) {
                    Ok(stored) => {
                        let mime = mime_guess::from_path(&stored.path)
                            .first_or_octet_stream()
                            .to_string();
                        let payload = serde_json::json!({
                            "imageId": stored.hash,
                            "path": stored.path,
                            "mimeType": mime
                        });
                        let _ = window_clone.emit("drag-drop-image", payload);
                    }
                    Err(e) => {
                        eprintln!("Failed to process dropped file: {}", e);
                    }
                }
            }
        }
        _ => {}
    });

    Ok(())
}
