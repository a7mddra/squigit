// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Error types for chat storage.

use thiserror::Error;

/// Storage error types.
#[derive(Error, Debug)]
pub enum StorageError {
    /// No data directory found.
    #[error("Could not find data directory")]
    NoDataDir,

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Empty image provided.
    #[error("Empty image data")]
    EmptyImage,

    /// Invalid hash format.
    #[error("Invalid hash format")]
    InvalidHash,

    /// Image not found.
    #[error("Image not found: {0}")]
    ImageNotFound(String),

    /// Chat not found.
    #[error("Chat not found: {0}")]
    ChatNotFound(String),

    /// Unsupported OCR model/frame key.
    #[error("Unsupported OCR model id: {0}")]
    InvalidOcrModel(String),
}

/// Result type alias for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;
