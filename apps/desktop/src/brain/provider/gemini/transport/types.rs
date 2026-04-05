// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "fileData", skip_serializing_if = "Option::is_none")]
    pub file_data: Option<GeminiFileData>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GeminiFunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    pub function_response: Option<GeminiFunctionResponse>,
    #[serde(rename = "thoughtSignature", skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFileData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "fileUri")]
    pub file_uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GeminiContent {
    pub(crate) role: String,
    pub(crate) parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GeminiRequest {
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    pub(crate) system_instruction: Option<GeminiContent>,
    pub(crate) contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub(crate) generation_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<Vec<serde_json::Value>>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    pub(crate) tool_config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeminiResponseCandidate {
    pub(crate) content: Option<GeminiResponseContent>,
    #[serde(rename = "finishReason")]
    pub(crate) finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeminiResponseContent {
    pub(crate) parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeminiResponsePart {
    pub(crate) text: Option<String>,
    #[serde(rename = "functionCall")]
    pub(crate) function_call: Option<GeminiFunctionCall>,
    #[serde(rename = "thoughtSignature")]
    pub(crate) thought_signature: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeminiResponseChunk {
    pub(crate) candidates: Option<Vec<GeminiResponseCandidate>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum GeminiEvent {
    Token {
        token: String,
    },
    Reset,
    ToolStatus {
        message: String,
    },
    ToolStart {
        id: String,
        name: String,
        args: serde_json::Value,
        message: String,
    },
    ToolEnd {
        id: String,
        name: String,
        status: String,
        result: serde_json::Value,
        message: String,
    },
}
