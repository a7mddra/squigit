// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_squigit_brain::events::BrainEventSink;
use ops_squigit_brain::service::{
    BrainService, CompressConversationRequest, GenerateChatTitleRequest,
    GenerateImageBriefRequest, StreamChatRequest,
};
use tauri::{AppHandle, Emitter};

pub struct DesktopBrainService {
    inner: BrainService,
}

impl DesktopBrainService {
    pub fn new() -> Self {
        Self {
            inner: BrainService::new(),
        }
    }

    pub async fn stream_chat(
        &self,
        app: AppHandle,
        request: StreamChatRequest,
    ) -> Result<(), String> {
        let sink = TauriEventSink { app };
        self.inner.stream_chat(&sink, request).await
    }

    pub async fn generate_chat_title(
        &self,
        request: GenerateChatTitleRequest,
    ) -> Result<String, String> {
        self.inner.generate_chat_title(request).await
    }

    pub async fn generate_image_brief(
        &self,
        request: GenerateImageBriefRequest,
    ) -> Result<String, String> {
        self.inner.generate_image_brief(request).await
    }

    pub async fn compress_conversation(
        &self,
        request: CompressConversationRequest,
    ) -> Result<String, String> {
        self.inner.compress_conversation(request).await
    }

    pub async fn cancel_request(&self, channel_id: Option<String>) -> Result<(), String> {
        self.inner.cancel_request(channel_id).await
    }

    pub async fn quick_answer_request(&self, channel_id: String) -> Result<(), String> {
        self.inner.request_quick_answer(channel_id).await
    }
}

impl Default for DesktopBrainService {
    fn default() -> Self {
        Self::new()
    }
}

struct TauriEventSink {
    app: AppHandle,
}

impl BrainEventSink for TauriEventSink {
    fn emit(&self, channel_id: &str, event: ops_squigit_brain::brain::provider::gemini::transport::types::GeminiEvent) {
        let _ = self.app.emit(channel_id, event);
    }
}
