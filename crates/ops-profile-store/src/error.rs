// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Error types for profile storage operations.

use std::io;
use thiserror::Error;

/// Result type alias for profile operations.
pub type Result<T> = std::result::Result<T, ProfileError>;

/// Errors that can occur during profile operations.
#[derive(Debug, Error)]
pub enum ProfileError {
    /// Failed to locate the user's config directory.
    #[error("Could not locate config directory")]
    NoConfigDir,

    /// Profile with the given ID was not found.
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    /// Cannot delete the last remaining profile.
    #[error("Cannot delete the last profile")]
    CannotDeleteLastProfile,

    /// Profile ID is invalid (empty or contains invalid characters).
    #[error("Invalid profile ID: {0}")]
    InvalidProfileId(String),

    /// API key provider name is unknown.
    #[error("Invalid API key provider: {0}")]
    InvalidProvider(String),

    /// Google authentication is not configured.
    #[error("{0}")]
    MissingCredentials(String),

    /// Authentication flow failed.
    #[error("{0}")]
    Auth(String),

    /// Security or encryption operation failed.
    #[error("{0}")]
    Security(String),

    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP or network failure during auth/avatar fetches.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Invalid URL encountered during auth flow construction/parsing.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}
