// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use tauri::State;

use crate::state::AppState;
use ops_chat_storage::{ChatStorage, StoredImage};
use ops_profile_store::ProfileStore;

/// Read image from clipboard and store in CAS.
/// Returns StoredImage { hash, path }.
#[tauri::command]
pub async fn read_clipboard_image(_state: State<'_, AppState>) -> Result<StoredImage, String> {
    use arboard::Clipboard;
    use image::ImageEncoder;

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

    let image_data = clipboard
        .get_image()
        .map_err(|e| format!("Failed to get image from clipboard: {}", e))?;

    let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        image_data.width as u32,
        image_data.height as u32,
        image_data.bytes.into_owned(),
    )
    .ok_or("Failed to create image buffer")?;

    let mut buffer = Vec::new();
    let cursor = std::io::Cursor::new(&mut buffer);

    image::codecs::png::PngEncoder::new(cursor)
        .write_image(
            &img,
            image_data.width as u32,
            image_data.height as u32,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| format!("Failed to encode image: {}", e))?;

    // Store in CAS using active profile's storage
    let profile_store = ProfileStore::new().map_err(|e| e.to_string())?;
    let active_id = profile_store
        .get_active_profile_id()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active profile. Please log in first.".to_string())?;

    let chats_dir = profile_store.get_chats_dir(&active_id);
    let storage = ChatStorage::with_base_dir(chats_dir).map_err(|e| e.to_string())?;
    let stored = storage.store_image(&buffer).map_err(|e| e.to_string())?;

    Ok(stored)
}

/// Read text from clipboard.
#[tauri::command]
pub async fn read_clipboard_text() -> Result<String, String> {
    use arboard::Clipboard;

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .get_text()
        .map_err(|e| format!("Failed to get text from clipboard: {}", e))
}

#[tauri::command]
pub async fn copy_image_to_clipboard(image_base64: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use base64::Engine;

    // Decode base64
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    let img =
        image::load_from_memory(&bytes).map_err(|e| format!("Failed to decode image: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
    };

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to copy image: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn copy_image_from_path_to_clipboard(path: String) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};

    let img = image::open(&path).map_err(|e| format!("Failed to open image at {}: {}", path, e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let img_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
    };

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_image(img_data)
        .map_err(|e| format!("Failed to copy image: {}", e))?;

    Ok(())
}
