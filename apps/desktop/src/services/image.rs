// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::services::tone::detect_image_tone_from_bytes;
use crate::state::AppState;
use ops_chat_storage::StoredImage;
use tauri::State;

pub fn process_and_store_image(
    path: String,
    state: &State<AppState>,
) -> Result<StoredImage, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
    process_bytes_internal(bytes, state)
}

pub fn process_bytes_internal(
    buffer: Vec<u8>,
    state: &State<AppState>,
) -> Result<StoredImage, String> {
    if buffer.is_empty() {
        return Err("Empty image buffer".to_string());
    }

    let explicit_tone = detect_image_tone_from_bytes(&buffer);
    let stored = ops_squigit_brain::image::process_bytes_internal(buffer, explicit_tone)?;

    let mut image_lock = state.image_data.lock();
    *image_lock = Some(stored.clone());

    Ok(stored)
}

pub async fn upload_image_to_imgbb(image_path: &str, api_key: &str) -> Result<String, String> {
    ops_squigit_brain::image::upload_image_to_imgbb(image_path, api_key).await
}
