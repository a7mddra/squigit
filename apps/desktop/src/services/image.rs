// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::state::AppState;
use ops_chat_storage::{ChatStorage, StoredImage};
use ops_profile_store::ProfileStore;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tauri::State;

pub fn process_and_store_image(
    path: String,
    state: &State<AppState>,
) -> Result<StoredImage, String> {
    let mut file = File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    process_bytes_internal(buffer, state)
}

pub fn process_bytes_internal(
    buffer: Vec<u8>,
    state: &State<AppState>,
) -> Result<StoredImage, String> {
    if buffer.is_empty() {
        return Err("Empty image buffer".to_string());
    }

    // Get active profile's chats directory for CAS storage
    let profile_store = ProfileStore::new().map_err(|e| e.to_string())?;
    let active_id = profile_store
        .get_active_profile_id()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active profile. Please log in first.".to_string())?;

    let chats_dir = profile_store.get_chats_dir(&active_id);
    let storage = ChatStorage::with_base_dir(chats_dir).map_err(|e| e.to_string())?;
    let stored = storage.store_image(&buffer).map_err(|e| e.to_string())?;

    let mut image_lock = state.image_data.lock();
    *image_lock = Some(stored.clone());

    Ok(stored)
}

#[derive(Debug, Deserialize)]
struct ImgBbUploadResponse {
    success: bool,
    data: Option<ImgBbUploadData>,
    error: Option<ImgBbUploadError>,
}

#[derive(Debug, Deserialize)]
struct ImgBbUploadData {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImgBbUploadError {
    message: Option<String>,
}

pub async fn upload_image_to_imgbb(image_path: &str, api_key: &str) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("ImgBB API key is required".to_string());
    }

    let path = Path::new(image_path);
    if !path.exists() {
        return Err(format!("Image file not found: {}", image_path));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", image_path));
    }

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read image file: {}", e))?;
    if bytes.is_empty() {
        return Err("Image file is empty".to_string());
    }

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("image");
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let image_part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str(mime.essence_str())
        .map_err(|e| format!("Failed to set MIME type for upload: {}", e))?;

    let form = reqwest::multipart::Form::new().part("image", image_part);

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.imgbb.com/1/upload")
        .query(&[("key", api_key)])
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("ImgBB upload request failed: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read ImgBB response: {}", e))?;

    if !status.is_success() {
        return Err(format!("ImgBB upload failed ({}): {}", status, body));
    }

    let parsed: ImgBbUploadResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse ImgBB response: {}", e))?;

    if !parsed.success {
        let message = parsed
            .error
            .and_then(|e| e.message)
            .unwrap_or_else(|| "ImgBB upload was not successful".to_string());
        return Err(message);
    }

    parsed
        .data
        .and_then(|d| d.url)
        .ok_or_else(|| "ImgBB response missing uploaded image URL".to_string())
}
