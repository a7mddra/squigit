// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::services::ocr::DesktopOcrService;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub id: String,
    pub path: String,
    pub size: u64,
}

#[tauri::command]
pub async fn download_ocr_model(
    state: tauri::State<'_, DesktopOcrService>,
    window: tauri::Window,
    url: String,
    model_id: String,
) -> Result<String, String> {
    println!("Downloading OCR model: {} -> {}", url, model_id);

    let path = state
        .download_model(&url, &model_id, |payload| {
            let _ = window.emit("download-progress", payload);
        })
        .await?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn cancel_download_ocr_model(
    state: tauri::State<'_, DesktopOcrService>,
    model_id: String,
) -> Result<(), String> {
    println!("Cancelling download for model: {}", model_id);
    state.cancel_model_download(&model_id);
    Ok(())
}

#[tauri::command]
pub fn list_downloaded_models(
    state: tauri::State<'_, DesktopOcrService>,
) -> Result<Vec<String>, String> {
    state.list_downloaded_models()
}

#[tauri::command]
pub fn get_model_path(
    state: tauri::State<'_, DesktopOcrService>,
    model_id: String,
) -> Result<String, String> {
    let path = state.get_model_dir(&model_id);
    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err(format!("Model {} not found", model_id))
    }
}
