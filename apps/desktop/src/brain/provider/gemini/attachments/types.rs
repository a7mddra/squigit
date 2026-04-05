// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileRef {
    pub file_uri: String,
    pub mime_type: String,
    pub display_name: String,
    pub uploaded_at: SystemTime,
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
