// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Type definitions for chat storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// Metadata for a chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMetadata {
    /// Unique identifier for the chat.
    pub id: String,
    /// Display title for the chat.
    pub title: String,
    /// When the chat was created.
    pub created_at: DateTime<Utc>,
    /// When the chat was last updated.
    pub updated_at: DateTime<Utc>,
    /// BLAKE3 hash of the associated image.
    pub image_hash: String,
    /// Whether the chat is pinned.
    #[serde(default)]
    pub is_pinned: bool,
    /// Whether the chat is starred (favorite).
    #[serde(default)]
    pub is_starred: bool,
    /// When the chat was pinned.
    #[serde(default)]
    pub pinned_at: Option<DateTime<Utc>>,
    pub ocr_lang: Option<String>,
    /// Image tone detected upon upload (light/dark).
    #[serde(default)]
    pub image_tone: Option<String>,
}

impl ChatMetadata {
    /// Create new thread metadata with a generated ID.
    pub fn new(title: String, image_hash: String, ocr_lang: Option<String>) -> Self {
        let now = Utc::now();
        // Generate ID: YYYYMMDD-HHMMSS-<UUID_SUFFIX>
        // Use first 8 chars of a UUID for randomness
        let date_part = now.format("%Y%m%d-%H%M%S").to_string();
        let uuid_part = Uuid::new_v4().to_string();
        let id = format!("{}-{}", date_part, &uuid_part[..8]);

        Self {
            id,
            title,
            created_at: now,
            updated_at: now,
            image_hash,
            is_pinned: false,
            is_starred: false,
            pinned_at: None,
            ocr_lang,
            image_tone: None,
        }
    }
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: "user" or "assistant".
    pub role: String,
    /// Message content (markdown).
    pub content: String,
    /// When the message was sent.
    pub timestamp: DateTime<Utc>,
    /// Optional structured citation chips shown under assistant responses.
    #[serde(default)]
    pub citations: Vec<CitationSource>,
    /// Optional tool call timeline metadata for this message.
    #[serde(default)]
    pub tool_steps: Vec<ToolStep>,
}

/// Structured citation source metadata persisted with a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationSource {
    pub title: String,
    pub url: String,
    pub summary: String,
    #[serde(default)]
    pub favicon: Option<String>,
}

/// Tool-step metadata persisted with a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStep {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub message: Option<String>,
}

impl ChatMessage {
    /// Create a new user message.
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
            timestamp: Utc::now(),
            citations: Vec::new(),
            tool_steps: Vec::new(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            timestamp: Utc::now(),
            citations: Vec::new(),
            tool_steps: Vec::new(),
        }
    }
}

/// OCR data for an image region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRegion {
    /// Extracted text.
    pub text: String,
    /// Bounding box coordinates.
    pub bbox: Vec<Vec<i32>>,
}

/// OCR frame: a map of model_id → cached OCR results.
/// `None` means the model hasn't scanned this image yet.
/// `Some(vec)` means cached results (may be empty if no text found).
pub type OcrFrame = HashMap<String, Option<Vec<OcrRegion>>>;

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Attachment kinds tracked per chat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatAttachmentKind {
    TextLocal,
    ImageUpload,
    DocumentUpload,
}

/// Persisted Gemini Files handle for a tracked attachment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatAttachmentGeminiFile {
    pub file_uri: String,
    pub file_name: String,
    pub mime_type: String,
    pub uploaded_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub last_validated_at: Option<DateTime<Utc>>,
}

/// Per-chat tracked attachment metadata keyed by CAS path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatAttachmentRecord {
    pub cas_path: String,
    pub display_name: String,
    pub kind: ChatAttachmentKind,
    pub mime_type: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub gemini_file: Option<ChatAttachmentGeminiFile>,
    #[serde(default = "now_utc")]
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    pub last_recalled_at: Option<DateTime<Utc>>,
}

/// Attachment registry persisted alongside chat data.
pub type AttachmentRegistry = BTreeMap<String, ChatAttachmentRecord>;

/// Complete chat data including messages and OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatData {
    /// Chat metadata.
    pub metadata: ChatMetadata,
    /// Chat messages.
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    /// OCR frame: keyed by model_id, each value is cached results or null.
    #[serde(default)]
    pub ocr_data: OcrFrame,
    /// Optional imgbb upload URL.
    #[serde(default)]
    pub imgbb_url: Option<String>,
    /// Rolling summary of compressed older conversation turns.
    /// Persisted so summaries survive app restarts and session switches.
    #[serde(default)]
    pub rolling_summary: Option<String>,
    /// Per-chat tracked attachments keyed by CAS path.
    #[serde(default)]
    pub attachment_registry: AttachmentRegistry,
    /// Generated concise text description of the session's startup image.
    #[serde(default)]
    pub image_brief: Option<String>,
}

impl ChatData {
    /// Create new thread data with metadata.
    pub fn new(metadata: ChatMetadata) -> Self {
        Self {
            metadata,
            messages: Vec::new(),
            ocr_data: HashMap::new(),
            imgbb_url: None,
            rolling_summary: None,
            attachment_registry: BTreeMap::new(),
            image_brief: None,
        }
    }
}

/// Result of storing an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredImage {
    /// BLAKE3 hash of the image (content ID).
    pub hash: String,
    /// Absolute path to the stored image file.
    pub path: String,
    /// Image tone detected upon upload (light/dark).
    #[serde(default)]
    pub tone: Option<String>,
}
