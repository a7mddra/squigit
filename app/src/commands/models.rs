// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::services::models::ModelManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub id: String,
    pub path: String,
    pub size: u64,
}

#[tauri::command]
pub async fn download_ocr_model(
    state: tauri::State<'_, ModelManager>,
    window: tauri::Window,
    url: String,
    model_id: String,
) -> Result<String, String> {
    println!("Downloading OCR model: {} -> {}", url, model_id);

    let path = state
        .download_and_extract(&url, &model_id, &window)
        .await
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn cancel_download_ocr_model(
    state: tauri::State<'_, ModelManager>,
    model_id: String,
) -> Result<(), String> {
    println!("Cancelling download for model: {}", model_id);
    state.cancel_download(&model_id);
    Ok(())
}

#[tauri::command]
pub fn list_downloaded_models(
    state: tauri::State<'_, ModelManager>,
) -> Result<Vec<String>, String> {
    let dir = state.get_model_dir("");

    let mut models = Vec::new();

    if dir.exists() {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                let has_graph =
                    path.join("inference.pdmodel").exists() || path.join("inference.json").exists();
                if has_graph && path.join("inference.pdiparams").exists() {
                    if let Some(name) = path.file_name() {
                        models.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    Ok(models)
}

#[tauri::command]
pub fn get_model_path(
    state: tauri::State<'_, ModelManager>,
    model_id: String,
) -> Result<String, String> {
    let path = state.get_model_dir(&model_id);
    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err(format!("Model {} not found", model_id))
    }
}
