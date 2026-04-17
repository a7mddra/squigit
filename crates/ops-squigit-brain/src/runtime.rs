// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::brain::provider::gemini::agent::request_control::GeminiRequestControl;
use crate::brain::provider::gemini::attachments::GeminiFileRef;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BrainRuntimeState {
    pub provider_file_cache: Arc<Mutex<HashMap<String, GeminiFileRef>>>,
    pub active_requests: Arc<Mutex<HashMap<String, GeminiRequestControl>>>,
}

impl BrainRuntimeState {
    pub fn new() -> Self {
        Self {
            provider_file_cache: Arc::new(Mutex::new(HashMap::new())),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for BrainRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}
