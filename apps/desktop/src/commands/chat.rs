// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Chat storage Tauri commands.

use crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal;
use crate::services::search::{search_local_chats, ChatSearchResult};
use crate::services::tone::detect_image_tone_from_bytes;
use ops_chat_storage::{
    ChatData, ChatMessage, ChatMetadata, ChatStorage, OcrFrame, OcrRegion, StoredImage,
};
use ops_profile_store::ProfileStore;

/// Helper to get storage for the active profile.
fn get_active_storage() -> Result<ChatStorage, String> {
    let profile_store = ProfileStore::new().map_err(|e| e.to_string())?;
    let active_id = profile_store
        .get_active_profile_id()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active profile. Please log in first.".to_string())?;

    let chats_dir = profile_store.get_chats_dir(&active_id);
    ChatStorage::with_base_dir(chats_dir).map_err(|e| e.to_string())
}

// =============================================================================
// Image Storage Commands
// =============================================================================

/// Store image bytes and return hash + path.
#[tauri::command]
pub fn store_image_bytes(bytes: Vec<u8>) -> Result<StoredImage, String> {
    let storage = get_active_storage()?;
    let explicit_tone = detect_image_tone_from_bytes(&bytes);
    storage
        .store_image(&bytes, explicit_tone)
        .map_err(|e| e.to_string())
}

/// Store image from file path and return hash + path.
#[tauri::command]
pub fn store_image_from_path(path: String) -> Result<StoredImage, String> {
    let storage = get_active_storage()?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let explicit_tone = detect_image_tone_from_bytes(&bytes);
    storage
        .store_image(&bytes, explicit_tone)
        .map_err(|e| e.to_string())
}

/// Store any file from path (preserving extension) and return hash + CAS path.
#[tauri::command]
pub fn store_file_from_path(path: String) -> Result<StoredImage, String> {
    let storage = get_active_storage()?;
    storage
        .store_file_from_path(&path, None)
        .map_err(|e| e.to_string())
}

/// Validate if a file is safe text (valid UTF-8 and no null bytes).
#[tauri::command]
pub fn validate_text_file(path: String) -> Result<bool, String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 8192]; // Read up to 8KB
    let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(bytes_read);

    // Empty files are valid text files
    if bytes_read == 0 {
        return Ok(true);
    }

    // Check for null bytes (quick binary check)
    if buffer.contains(&0) {
        return Ok(false);
    }

    // Check strict UTF-8 validity
    match std::str::from_utf8(&buffer) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Get the path to a stored image by its hash.
#[tauri::command]
pub fn get_image_path(hash: String) -> Result<String, String> {
    let storage = get_active_storage()?;
    storage.get_image_path(&hash).map_err(|e| e.to_string())
}

/// Resolve an attachment path (absolute or relative CAS path) to an absolute path.
#[tauri::command]
pub fn resolve_attachment_path(path: String) -> Result<String, String> {
    let resolved = resolve_attachment_path_internal(&path)?;
    Ok(resolved.to_string_lossy().to_string())
}

/// Detect image tone for a given attachment path.
#[tauri::command]
pub fn detect_image_tone(path: String) -> Result<String, String> {
    let resolved = resolve_attachment_path_internal(&path)?;
    let bytes = std::fs::read(resolved).map_err(|e| e.to_string())?;

    match detect_image_tone_from_bytes(&bytes).as_deref() {
        Some("l") => Ok("light".to_string()),
        Some("d") => Ok("dark".to_string()),
        Some(other) => Ok(other.to_string()),
        None => Err("Failed to detect image tone".to_string()),
    }
}

/// Read UTF-8 text content from an attachment path.
#[tauri::command]
pub fn read_attachment_text(path: String) -> Result<String, String> {
    let resolved = resolve_attachment_path_internal(&path)?;
    let bytes = std::fs::read(resolved).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Reveal a file in the system file manager, selecting it when possible.
#[tauri::command]
pub fn reveal_in_file_manager(path: String) -> Result<(), String> {
    use std::process::Command;

    let resolved = resolve_attachment_path_internal(&path)?;

    #[cfg(target_os = "windows")]
    {
        let target = resolved.to_string_lossy().to_string();
        Command::new("explorer")
            .arg(format!(r#"/select,"{}""#, target))
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(&resolved)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let target = resolved.to_string_lossy().to_string();
        let parent = resolved
            .parent()
            .ok_or_else(|| "No parent directory".to_string())?
            .to_string_lossy()
            .to_string();

        let select_candidates: Vec<(&str, Vec<String>)> = vec![
            ("nautilus", vec!["--select".into(), target.clone()]),
            ("nemo", vec!["--select".into(), target.clone()]),
            ("caja", vec!["--select".into(), target.clone()]),
            ("dolphin", vec!["--select".into(), target.clone()]),
            ("konqueror", vec![target.clone()]),
            ("thunar", vec!["--select".into(), target.clone()]),
            ("pcmanfm-qt", vec!["--select".into(), target.clone()]),
            ("pcmanfm", vec!["--select".into(), target.clone()]),
            ("spacefm", vec!["--select".into(), target.clone()]),
            ("pantheon-files", vec![target.clone()]),
            ("doublecmd", vec![target.clone()]),
            ("krusader", vec![target.clone()]),
            ("xfe", vec![target.clone()]),
        ];

        for (bin, args) in select_candidates {
            if Command::new(bin).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }

        let parent_candidates: Vec<(&str, Vec<String>)> = vec![
            ("xdg-open", vec![parent.clone()]),
            ("gio", vec!["open".into(), parent.clone()]),
            ("exo-open", vec![parent.clone()]),
            ("kde-open5", vec![parent.clone()]),
            ("kde-open", vec![parent.clone()]),
            ("gnome-open", vec![parent.clone()]),
            ("pcmanfm", vec![parent.clone()]),
            ("thunar", vec![parent.clone()]),
            ("nemo", vec![parent.clone()]),
            ("caja", vec![parent.clone()]),
            ("dolphin", vec![parent.clone()]),
            ("nautilus", vec![parent.clone()]),
            ("pantheon-files", vec![parent.clone()]),
        ];

        for (bin, args) in parent_candidates {
            if Command::new(bin).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }

        return Err("Failed to open a file manager on this Linux environment".to_string());
    }

    #[allow(unreachable_code)]
    Err("Unsupported platform".to_string())
}

// =============================================================================
// Chat Storage Commands
// =============================================================================

/// Create a new thread with the given image hash.
#[tauri::command]
pub fn create_chat(
    title: String,
    image_hash: String,
    ocr_lang: Option<String>,
) -> Result<ChatMetadata, String> {
    let storage = get_active_storage()?;
    let mut metadata = ChatMetadata::new(title, image_hash.clone(), ocr_lang);
    metadata.image_tone = storage.get_image_tone(&image_hash);
    let chat = ChatData::new(metadata.clone());
    storage.save_chat(&chat).map_err(|e| e.to_string())?;
    Ok(metadata)
}

/// Load a chat by ID.
#[tauri::command]
pub fn load_chat(chat_id: String) -> Result<ChatData, String> {
    let storage = get_active_storage()?;
    storage.load_chat(&chat_id).map_err(|e| e.to_string())
}

/// List all chats (metadata only).
#[tauri::command]
pub fn list_chats() -> Result<Vec<ChatMetadata>, String> {
    let storage = get_active_storage()?;
    storage.list_chats().map_err(|e| e.to_string())
}

/// Search chats and return ranked message hits.
#[tauri::command]
pub fn search_chats(query: String, limit: Option<usize>) -> Result<Vec<ChatSearchResult>, String> {
    let storage = get_active_storage()?;
    let max_results = limit.unwrap_or(60).clamp(1, 200);
    search_local_chats(&storage, &query, max_results)
}

/// Delete a chat by ID.
#[tauri::command]
pub fn delete_chat(chat_id: String) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage.delete_chat(&chat_id).map_err(|e| e.to_string())
}

/// Update chat metadata (rename, pin, star, etc.).
#[tauri::command]
pub fn update_chat_metadata(metadata: ChatMetadata) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage
        .update_chat_metadata(&metadata)
        .map_err(|e| e.to_string())
}

// =============================================================================
// Message Commands
// =============================================================================

#[tauri::command]
pub fn append_chat_message(chat_id: String, role: String, content: String) -> Result<(), String> {
    let storage = get_active_storage()?;
    let message = if role == "user" {
        ChatMessage::user(content)
    } else {
        ChatMessage::assistant(content)
    };
    storage
        .append_message(&chat_id, &message)
        .map_err(|e| e.to_string())
}

/// Overwrite all messages in a chat.
#[tauri::command]
pub fn overwrite_chat_messages(chat_id: String, messages: Vec<ChatMessage>) -> Result<(), String> {
    let storage = get_active_storage()?;
    let mut chat = storage.load_chat(&chat_id).map_err(|e| e.to_string())?;
    chat.messages = messages;
    storage.save_chat(&chat).map_err(|e| e.to_string())
}

// =============================================================================
// OCR Commands
// =============================================================================

/// Save OCR data for a specific model.
#[tauri::command]
pub fn save_ocr_data(
    chat_id: String,
    model_id: String,
    ocr_data: Vec<OcrRegion>,
) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage
        .save_ocr_data(&chat_id, &model_id, &ocr_data)
        .map_err(|e| e.to_string())
}

/// Get OCR data for a specific model.
#[tauri::command]
pub fn get_ocr_data(chat_id: String, model_id: String) -> Result<Option<Vec<OcrRegion>>, String> {
    let storage = get_active_storage()?;
    storage
        .get_ocr_data(&chat_id, &model_id)
        .map_err(|e| e.to_string())
}

/// Get the entire OCR frame for a chat.
#[tauri::command]
pub fn get_ocr_frame(chat_id: String) -> Result<OcrFrame, String> {
    let storage = get_active_storage()?;
    storage.get_ocr_frame(&chat_id).map_err(|e| e.to_string())
}

/// Initialize OCR frame with null values for given model IDs.
#[tauri::command]
pub fn init_ocr_frame(chat_id: String, model_ids: Vec<String>) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage
        .init_ocr_frame(&chat_id, &model_ids)
        .map_err(|e| e.to_string())
}

// =============================================================================
// ImgBB Commands
// =============================================================================

/// Save imgbb URL for a chat.
#[tauri::command]
pub fn save_imgbb_url(chat_id: String, url: String) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage
        .save_imgbb_url(&chat_id, &url)
        .map_err(|e| e.to_string())
}

/// Get imgbb URL for a chat.
#[tauri::command]
pub fn get_imgbb_url(chat_id: String) -> Result<Option<String>, String> {
    let storage = get_active_storage()?;
    storage.get_imgbb_url(&chat_id).map_err(|e| e.to_string())
}

/// Save rolling summary for a chat.
#[tauri::command]
pub fn save_rolling_summary(chat_id: String, summary: String) -> Result<(), String> {
    let storage = get_active_storage()?;
    storage
        .save_rolling_summary(&chat_id, &summary)
        .map_err(|e| e.to_string())
}
