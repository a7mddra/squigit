// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileRef {
    pub file_uri: String,
    pub file_name: String,
    pub mime_type: String,
    pub display_name: String,
    pub uploaded_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GeminiFileObject {
    pub(super) name: Option<String>,
    pub(super) uri: Option<String>,
    pub(super) state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GeminiFileUploadFinalizeResponse {
    pub(super) file: GeminiFileObject,
}
