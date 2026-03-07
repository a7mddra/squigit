// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_chat_storage::StoredImage;
use parking_lot::Mutex;
use std::sync::{atomic::AtomicBool, Arc};

/// Handle to a running OCR job for cancellation support.
pub struct OcrJobHandle {
    /// Child process handle for kill-based cancellation.
    pub child: tokio::process::Child,
}

pub struct AppState {
    pub image_data: Arc<Mutex<Option<StoredImage>>>,
    pub auth_running: Arc<AtomicBool>,
    /// Flag set when auth is cancelled or timed out - prevents late auth from succeeding
    pub auth_cancelled: Arc<AtomicBool>,
    /// Active OCR job handle for cancellation support.
    pub ocr_job: Arc<tokio::sync::Mutex<Option<OcrJobHandle>>>,
    pub gemini_file_cache: Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<String, crate::commands::gemini_files::GeminiFileRef>,
        >,
    >,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            image_data: Arc::new(Mutex::new(None)),
            auth_running: Arc::new(AtomicBool::new(false)),
            auth_cancelled: Arc::new(AtomicBool::new(false)),
            ocr_job: Arc::new(tokio::sync::Mutex::new(None)),
            gemini_file_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
