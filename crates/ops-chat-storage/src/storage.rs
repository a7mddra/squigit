// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Content Addressable Storage (CAS) implementation for images and chat data.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::error::{Result, StorageError};
use crate::types::{ChatData, ChatMessage, ChatMetadata, OcrFrame, OcrRegion, StoredImage};

const DEFAULT_OCR_MODEL_ID: &str = "pp-ocr-v5-en";
const AUTO_OCR_DISABLED_MODEL_ID: &str = "__meta_auto_ocr_disabled__";

fn is_supported_ocr_model_id(model_id: &str) -> bool {
    matches!(
        model_id,
        "pp-ocr-v5-en"
            | "pp-ocr-v5-latin"
            | "pp-ocr-v5-cyrillic"
            | "pp-ocr-v5-korean"
            | "pp-ocr-v5-cjk"
            | "pp-ocr-v5-devanagari"
    )
}

fn normalize_ocr_model_id(model_id: &str) -> &str {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return DEFAULT_OCR_MODEL_ID;
    }
    if is_supported_ocr_model_id(trimmed) {
        return trimmed;
    }
    DEFAULT_OCR_MODEL_ID
}

fn is_reserved_ocr_frame_id(model_id: &str) -> bool {
    model_id == AUTO_OCR_DISABLED_MODEL_ID
}

fn canonicalize_ocr_frame_id(model_id: &str) -> Option<&str> {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    if is_supported_ocr_model_id(trimmed) || is_reserved_ocr_frame_id(trimmed) {
        return Some(trimmed);
    }
    None
}

fn retain_supported_ocr_frame_ids(frame: &mut OcrFrame) -> bool {
    let keys: Vec<String> = frame.keys().cloned().collect();
    let mut changed = false;

    for key in keys {
        if !is_supported_ocr_model_id(&key) && !is_reserved_ocr_frame_id(&key) {
            frame.remove(&key);
            changed = true;
        }
    }

    changed
}

/// Main storage manager for chats and images.
pub struct ChatStorage {
    /// Base directory for all storage.
    base_dir: PathBuf,
    /// Directory for CAS objects (images).
    objects_dir: PathBuf,
    /// Path to the chat index file.
    index_path: PathBuf,
}

impl ChatStorage {
    /// Create a new storage manager with a custom base directory.
    ///
    /// This is the primary constructor for profile-aware storage.
    /// Use this with a profile's chats directory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ops_chat_storage::ChatStorage;
    /// use std::path::PathBuf;
    ///
    /// let profile_chats_dir = PathBuf::from("/path/to/profile/chats");
    /// let storage = ChatStorage::with_base_dir(profile_chats_dir).unwrap();
    /// ```
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        let objects_dir = base_dir.join("objects");
        let index_path = base_dir.join("index.json");

        // Create directories if they don't exist
        fs::create_dir_all(&objects_dir)?;

        Ok(Self {
            base_dir,
            objects_dir,
            index_path,
        })
    }

    /// Create a new storage manager using the default location.
    ///
    /// Uses `~/.config/squigit/chats/` on Linux (and appropriate config dirs on other OSs).
    ///
    /// **Note**: This constructor is provided for backward compatibility.
    /// New code should use `with_base_dir()` with a profile-specific path.
    #[deprecated(
        since = "0.2.0",
        note = "Use with_base_dir() for profile-aware storage"
    )]
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .ok_or(StorageError::NoDataDir)?
            .join("Squigit".to_lowercase())
            .join("chats");

        Self::with_base_dir(base_dir)
    }

    /// Get the base storage directory path.
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Get the objects directory path.
    pub fn objects_dir(&self) -> &PathBuf {
        &self.objects_dir
    }

    // =========================================================================
    // Image Storage (CAS)
    // =========================================================================

    /// Store image bytes using content-addressable storage.
    ///
    /// Returns the hash and path to the stored image.
    /// If the image already exists (same hash), returns the existing path.
    pub fn store_image(&self, bytes: &[u8]) -> Result<StoredImage> {
        if bytes.is_empty() {
            return Err(StorageError::EmptyImage);
        }

        // Compute BLAKE3 hash
        let hash = blake3::hash(bytes).to_hex().to_string();

        // Create subdirectory using first 2 chars of hash
        let prefix = &hash[..2];
        let subdir = self.objects_dir.join(prefix);
        fs::create_dir_all(&subdir)?;

        // Full path: objects/<prefix>/<hash>.png
        let file_path = subdir.join(format!("{}.png", hash));

        // Only write if file doesn't exist (deduplication)
        if !file_path.exists() {
            let mut file = File::create(&file_path)?;
            file.write_all(bytes)?;
        }

        Ok(StoredImage {
            hash,
            path: file_path.to_string_lossy().to_string(),
        })
    }

    /// Store an image from a file path.
    pub fn store_image_from_path(&self, path: &str) -> Result<StoredImage> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        self.store_image(&buffer)
    }

    /// Store a generic file using content-addressable storage, preserving the original extension.
    ///
    /// Returns the hash and path to the stored file.
    /// If the file already exists (same hash + extension), returns the existing path.
    pub fn store_file(&self, bytes: &[u8], extension: &str) -> Result<StoredImage> {
        if bytes.is_empty() {
            return Err(StorageError::EmptyImage);
        }

        let hash = blake3::hash(bytes).to_hex().to_string();

        let prefix = &hash[..2];
        let subdir = self.objects_dir.join(prefix);
        fs::create_dir_all(&subdir)?;

        let ext = if extension.is_empty() {
            "bin"
        } else {
            extension
        };
        let file_path = subdir.join(format!("{}.{}", hash, ext));

        if !file_path.exists() {
            let mut file = File::create(&file_path)?;
            file.write_all(bytes)?;
        }

        Ok(StoredImage {
            hash,
            path: file_path.to_string_lossy().to_string(),
        })
    }

    /// Store a file from a filesystem path, preserving the original extension.
    pub fn store_file_from_path(&self, path: &str) -> Result<StoredImage> {
        let source = std::path::Path::new(path);
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        self.store_file(&buffer, &extension)
    }

    /// Get the path to a stored image by its hash.
    pub fn get_image_path(&self, hash: &str) -> Result<String> {
        let prefix = hash.get(..2).ok_or(StorageError::InvalidHash)?;
        let file_path = self.objects_dir.join(prefix).join(format!("{}.png", hash));

        if file_path.exists() {
            Ok(file_path.to_string_lossy().to_string())
        } else {
            Err(StorageError::ImageNotFound(hash.to_string()))
        }
    }

    // =========================================================================
    // Chat Storage
    // =========================================================================

    /// Get the directory for a specific chat.
    fn chat_dir(&self, chat_id: &str) -> PathBuf {
        self.base_dir.join(chat_id)
    }

    /// Save a new thread or update an existing one.
    pub fn save_chat(&self, chat: &ChatData) -> Result<()> {
        let chat_dir = self.chat_dir(&chat.metadata.id);
        fs::create_dir_all(&chat_dir)?;

        // Save metadata
        let meta_path = chat_dir.join("meta.json");
        let meta_json = serde_json::to_string_pretty(&chat.metadata)?;
        fs::write(&meta_path, meta_json)?;

        // Always save OCR frame file
        let ocr_path = chat_dir.join("ocr_frame.json");
        let ocr_json = serde_json::to_string_pretty(&chat.ocr_data)?;
        fs::write(&ocr_path, ocr_json)?;

        // Save messages file
        let messages_path = chat_dir.join("messages.md");
        if !chat.messages.is_empty() {
            let md_content = self.messages_to_markdown(&chat.messages);
            fs::write(&messages_path, md_content)?;
        } else if messages_path.exists() {
            fs::remove_file(&messages_path)?;
        }

        // Save imgbb URL if present
        if let Some(ref url) = chat.imgbb_url {
            let url_path = chat_dir.join("imgbb_url.txt");
            fs::write(&url_path, url)?;
        }

        // Update the index
        self.update_index(&chat.metadata)?;

        Ok(())
    }

    /// Load a chat by ID.
    pub fn load_chat(&self, chat_id: &str) -> Result<ChatData> {
        let chat_dir = self.chat_dir(chat_id);

        if !chat_dir.exists() {
            return Err(StorageError::ChatNotFound(chat_id.to_string()));
        }

        // Load metadata
        let meta_path = chat_dir.join("meta.json");
        let meta_json = fs::read_to_string(&meta_path)?;
        let mut metadata: ChatMetadata = serde_json::from_str(&meta_json)?;
        let mut metadata_changed = false;
        if let Some(lang) = metadata.ocr_lang.clone() {
            let normalized_lang = normalize_ocr_model_id(&lang).to_string();
            if normalized_lang != lang {
                metadata.ocr_lang = Some(normalized_lang);
                metadata_changed = true;
            }
        }

        // Load OCR frame (supports one-time conversion from old ocr.json).
        let frame_path = chat_dir.join("ocr_frame.json");
        let old_frame_path = chat_dir.join("ocr.json");
        let mut frame_changed = false;
        let mut ocr_data: OcrFrame = if frame_path.exists() {
            let json = fs::read_to_string(&frame_path)?;
            serde_json::from_str(&json)?
        } else if old_frame_path.exists() {
            // Convert old flat array into frame format keyed by default model id.
            let json = fs::read_to_string(&old_frame_path)?;
            let old_regions: Vec<OcrRegion> = serde_json::from_str(&json).unwrap_or_default();
            let mut frame = OcrFrame::new();
            if !old_regions.is_empty() {
                frame.insert(DEFAULT_OCR_MODEL_ID.to_string(), Some(old_regions));
            }
            frame_changed = true;
            let _ = fs::remove_file(&old_frame_path);
            frame
        } else {
            OcrFrame::new()
        };

        if retain_supported_ocr_frame_ids(&mut ocr_data) {
            frame_changed = true;
        }
        if frame_changed {
            let new_json = serde_json::to_string_pretty(&ocr_data)?;
            fs::write(&frame_path, new_json)?;
        }
        if metadata_changed {
            let new_meta = serde_json::to_string_pretty(&metadata)?;
            fs::write(&meta_path, new_meta)?;
            self.update_index(&metadata)?;
        }

        // Load messages from markdown
        let messages_path = chat_dir.join("messages.md");
        let messages = if messages_path.exists() {
            let md_content = fs::read_to_string(&messages_path)?;
            self.markdown_to_messages(&md_content)
        } else {
            Vec::new()
        };

        // Load imgbb URL
        let url_path = chat_dir.join("imgbb_url.txt");
        let imgbb_url = if url_path.exists() {
            Some(fs::read_to_string(&url_path)?)
        } else {
            None
        };

        Ok(ChatData {
            metadata,
            messages,
            ocr_data,
            imgbb_url,
        })
    }

    /// List all chats (metadata only).
    pub fn list_chats(&self) -> Result<Vec<ChatMetadata>> {
        if !self.index_path.exists() {
            return Ok(Vec::new());
        }

        let index_json = fs::read_to_string(&self.index_path)?;
        let chats: Vec<ChatMetadata> = serde_json::from_str(&index_json)?;
        Ok(chats)
    }

    /// Delete a chat by ID.
    pub fn delete_chat(&self, chat_id: &str) -> Result<()> {
        let chat_dir = self.chat_dir(chat_id);

        if chat_dir.exists() {
            fs::remove_dir_all(&chat_dir)?;
        }

        // Update index to remove the chat
        self.remove_from_index(chat_id)?;

        Ok(())
    }

    /// Update chat metadata (for rename, pin, star, etc.).
    pub fn update_chat_metadata(&self, metadata: &ChatMetadata) -> Result<()> {
        let chat_dir = self.chat_dir(&metadata.id);

        if !chat_dir.exists() {
            return Err(StorageError::ChatNotFound(metadata.id.clone()));
        }

        // Save updated metadata
        let meta_path = chat_dir.join("meta.json");
        let meta_json = serde_json::to_string_pretty(metadata)?;
        fs::write(&meta_path, meta_json)?;

        // Update index
        self.update_index(metadata)?;

        Ok(())
    }

    // =========================================================================
    // OCR and ImgBB
    // =========================================================================

    /// Save OCR data for a specific model into the chat's OCR frame.
    /// Merges the data into the existing frame (read-modify-write).
    pub fn save_ocr_data(
        &self,
        chat_id: &str,
        model_id: &str,
        ocr_data: &[OcrRegion],
    ) -> Result<()> {
        let chat_dir = self.chat_dir(chat_id);
        fs::create_dir_all(&chat_dir)?;
        let canonical_model_id = canonicalize_ocr_frame_id(model_id)
            .ok_or_else(|| StorageError::InvalidOcrModel(model_id.to_string()))?;

        let frame_path = chat_dir.join("ocr_frame.json");
        let mut frame: OcrFrame = if frame_path.exists() {
            let json = fs::read_to_string(&frame_path)?;
            serde_json::from_str(&json)?
        } else {
            OcrFrame::new()
        };
        retain_supported_ocr_frame_ids(&mut frame);

        frame.insert(canonical_model_id.to_string(), Some(ocr_data.to_vec()));

        let json = serde_json::to_string_pretty(&frame)?;
        fs::write(&frame_path, json)?;

        Ok(())
    }

    /// Get OCR data for a specific model from the chat's frame.
    /// Returns None if the model hasn't scanned yet, or empty vec if no frame exists.
    pub fn get_ocr_data(&self, chat_id: &str, model_id: &str) -> Result<Option<Vec<OcrRegion>>> {
        let frame_path = self.chat_dir(chat_id).join("ocr_frame.json");
        let canonical_model_id = canonicalize_ocr_frame_id(model_id)
            .ok_or_else(|| StorageError::InvalidOcrModel(model_id.to_string()))?;

        if !frame_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&frame_path)?;
        let mut frame: OcrFrame = serde_json::from_str(&json)?;
        if retain_supported_ocr_frame_ids(&mut frame) {
            let normalized = serde_json::to_string_pretty(&frame)?;
            fs::write(&frame_path, normalized)?;
        }
        Ok(frame.get(canonical_model_id).cloned().unwrap_or(None))
    }

    /// Get the entire OCR frame for a chat.
    pub fn get_ocr_frame(&self, chat_id: &str) -> Result<OcrFrame> {
        let frame_path = self.chat_dir(chat_id).join("ocr_frame.json");

        if !frame_path.exists() {
            return Ok(OcrFrame::new());
        }

        let json = fs::read_to_string(&frame_path)?;
        let mut frame: OcrFrame = serde_json::from_str(&json)?;
        if retain_supported_ocr_frame_ids(&mut frame) {
            let normalized = serde_json::to_string_pretty(&frame)?;
            fs::write(&frame_path, normalized)?;
        }
        Ok(frame)
    }

    /// Initialize an OCR frame with null values for all given model IDs.
    /// Only adds keys that don't already exist (won't overwrite cached data).
    pub fn init_ocr_frame(&self, chat_id: &str, model_ids: &[String]) -> Result<()> {
        let chat_dir = self.chat_dir(chat_id);
        fs::create_dir_all(&chat_dir)?;

        let frame_path = chat_dir.join("ocr_frame.json");
        let mut frame: OcrFrame = if frame_path.exists() {
            let json = fs::read_to_string(&frame_path)?;
            serde_json::from_str(&json)?
        } else {
            OcrFrame::new()
        };
        retain_supported_ocr_frame_ids(&mut frame);

        for model_id in model_ids {
            let canonical_model_id = canonicalize_ocr_frame_id(model_id)
                .ok_or_else(|| StorageError::InvalidOcrModel(model_id.clone()))?;
            frame.entry(canonical_model_id.to_string()).or_insert(None);
        }

        let json = serde_json::to_string_pretty(&frame)?;
        fs::write(&frame_path, json)?;

        Ok(())
    }

    /// Save imgbb URL for a chat.
    pub fn save_imgbb_url(&self, chat_id: &str, url: &str) -> Result<()> {
        let chat_dir = self.chat_dir(chat_id);
        fs::create_dir_all(&chat_dir)?;

        let url_path = chat_dir.join("imgbb_url.txt");
        fs::write(&url_path, url)?;

        Ok(())
    }

    /// Get imgbb URL for a chat.
    pub fn get_imgbb_url(&self, chat_id: &str) -> Result<Option<String>> {
        let url_path = self.chat_dir(chat_id).join("imgbb_url.txt");

        if !url_path.exists() {
            return Ok(None);
        }

        let url = fs::read_to_string(&url_path)?;
        Ok(Some(url))
    }

    /// Append a message to a chat.
    pub fn append_message(&self, chat_id: &str, message: &ChatMessage) -> Result<()> {
        let chat_dir = self.chat_dir(chat_id);
        fs::create_dir_all(&chat_dir)?;

        let messages_path = chat_dir.join("messages.md");

        // Append to existing file or create new
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&messages_path)?;

        let md_entry = self.message_to_markdown(message);
        file.write_all(md_entry.as_bytes())?;

        // Update the chat's updated_at timestamp
        let meta_path = chat_dir.join("meta.json");
        if meta_path.exists() {
            let meta_json = fs::read_to_string(&meta_path)?;
            let mut metadata: ChatMetadata = serde_json::from_str(&meta_json)?;
            metadata.updated_at = chrono::Utc::now();
            let updated_json = serde_json::to_string_pretty(&metadata)?;
            fs::write(&meta_path, updated_json)?;
            self.update_index(&metadata)?;
        }

        Ok(())
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Update the index with chat metadata.
    fn update_index(&self, metadata: &ChatMetadata) -> Result<()> {
        let mut chats = self.list_chats().unwrap_or_default();

        // Remove existing entry if present
        chats.retain(|c| c.id != metadata.id);

        // Add updated entry
        chats.push(metadata.clone());

        // Sort by updated_at descending
        chats.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let json = serde_json::to_string_pretty(&chats)?;
        fs::write(&self.index_path, json)?;

        Ok(())
    }

    /// Remove a chat from the index.
    fn remove_from_index(&self, chat_id: &str) -> Result<()> {
        let mut chats = self.list_chats().unwrap_or_default();
        chats.retain(|c| c.id != chat_id);

        let json = serde_json::to_string_pretty(&chats)?;
        fs::write(&self.index_path, json)?;

        Ok(())
    }

    /// Convert messages to markdown format.
    fn messages_to_markdown(&self, messages: &[ChatMessage]) -> String {
        messages
            .iter()
            .map(|m| self.message_to_markdown(m))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Convert a single message to markdown.
    fn message_to_markdown(&self, message: &ChatMessage) -> String {
        let role_label = if message.role == "user" {
            "## User"
        } else {
            "## Assistant"
        };

        format!(
            "{}\n<!-- {} -->\n\n{}\n\n",
            role_label,
            message.timestamp.to_rfc3339(),
            message.content
        )
    }

    /// Parse markdown back to messages.
    fn markdown_to_messages(&self, content: &str) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        let mut current_role: Option<String> = None;
        let mut current_timestamp: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut current_content = String::new();

        for line in content.lines() {
            if line.starts_with("## User") {
                // Save previous message if any
                if let Some(role) = current_role.take() {
                    messages.push(ChatMessage {
                        role,
                        content: current_content.trim().to_string(),
                        timestamp: current_timestamp.unwrap_or_else(chrono::Utc::now),
                    });
                }
                current_role = Some("user".to_string());
                current_content.clear();
                current_timestamp = None;
            } else if line.starts_with("## Assistant") {
                // Save previous message if any
                if let Some(role) = current_role.take() {
                    messages.push(ChatMessage {
                        role,
                        content: current_content.trim().to_string(),
                        timestamp: current_timestamp.unwrap_or_else(chrono::Utc::now),
                    });
                }
                current_role = Some("assistant".to_string());
                current_content.clear();
                current_timestamp = None;
            } else if line.starts_with("<!-- ") && line.ends_with(" -->") {
                // Parse timestamp from comment
                let ts_str = &line[5..line.len() - 4];
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                    current_timestamp = Some(ts.with_timezone(&chrono::Utc));
                }
            } else if current_role.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Save last message
        if let Some(role) = current_role {
            messages.push(ChatMessage {
                role,
                content: current_content.trim().to_string(),
                timestamp: current_timestamp.unwrap_or_else(chrono::Utc::now),
            });
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_storage() -> (ChatStorage, PathBuf) {
        let base_dir =
            std::env::temp_dir().join(format!("squigit-ocr-storage-test-{}", uuid::Uuid::new_v4()));
        let storage = ChatStorage::with_base_dir(base_dir.clone()).expect("storage init");
        (storage, base_dir)
    }

    #[test]
    fn auto_ocr_disabled_key_is_preserved_and_does_not_overwrite_english() {
        let (storage, base_dir) = make_test_storage();
        let metadata = ChatMetadata::new("Test".to_string(), "0".repeat(64), None);
        let chat = ChatData::new(metadata.clone());
        storage.save_chat(&chat).expect("save chat");

        let en_regions = vec![OcrRegion {
            text: "hello".to_string(),
            bbox: vec![vec![0, 0], vec![10, 0], vec![10, 10], vec![0, 10]],
        }];

        storage
            .save_ocr_data(&metadata.id, DEFAULT_OCR_MODEL_ID, &en_regions)
            .expect("save english ocr");
        storage
            .save_ocr_data(&metadata.id, AUTO_OCR_DISABLED_MODEL_ID, &[])
            .expect("save auto-disable marker");

        let frame = storage.get_ocr_frame(&metadata.id).expect("load frame");
        let en = frame
            .get(DEFAULT_OCR_MODEL_ID)
            .and_then(|v| v.as_ref())
            .expect("english ocr present");
        assert_eq!(en.len(), 1);

        let auto_disabled = frame
            .get(AUTO_OCR_DISABLED_MODEL_ID)
            .and_then(|v| v.as_ref())
            .expect("auto-disable marker present");
        assert!(auto_disabled.is_empty());

        let _ = std::fs::remove_dir_all(base_dir);
    }

    #[test]
    fn invalid_ocr_model_id_returns_error() {
        let (storage, base_dir) = make_test_storage();
        let result = storage.save_ocr_data("chat-1", "bogus-model", &[]);

        assert!(matches!(result, Err(StorageError::InvalidOcrModel(_))));

        let _ = std::fs::remove_dir_all(base_dir);
    }
}
