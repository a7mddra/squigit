// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::services::brain::DesktopBrainService;
use ops_squigit_brain::service::{
    CompressConversationRequest, GenerateChatTitleRequest, GenerateImageBriefRequest,
    StreamChatRequest,
};
use tauri::{AppHandle, State};

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn stream_chat(
    app: AppHandle,
    brain: State<'_, DesktopBrainService>,
    api_key: String,
    model: String,
    is_initial_turn: bool,
    image_path: Option<String>,
    image_description: Option<String>,
    user_first_msg: Option<String>,
    history_log: Option<String>,
    rolling_summary: Option<String>,
    user_message: String,
    channel_id: String,
    chat_id: Option<String>,
    user_name: Option<String>,
    user_email: Option<String>,
    user_instruction: Option<String>,
    image_brief: Option<String>,
) -> Result<(), String> {
    brain
        .stream_chat(
            app,
            StreamChatRequest {
                api_key,
                model,
                is_initial_turn,
                image_path,
                image_description,
                user_first_msg,
                history_log,
                rolling_summary,
                user_message,
                channel_id,
                chat_id,
                user_name,
                user_email,
                user_instruction,
                image_brief,
            },
        )
        .await
}

#[tauri::command]
pub async fn generate_chat_title(
    brain: State<'_, DesktopBrainService>,
    api_key: String,
    model: String,
    prompt_context: String,
) -> Result<String, String> {
    brain
        .generate_chat_title(GenerateChatTitleRequest {
            api_key,
            model,
            prompt_context,
        })
        .await
}

#[tauri::command]
pub async fn generate_image_brief(
    brain: State<'_, DesktopBrainService>,
    api_key: String,
    image_path: String,
    model: Option<String>,
) -> Result<String, String> {
    brain
        .generate_image_brief(GenerateImageBriefRequest {
            api_key,
            image_path,
            model,
        })
        .await
}

#[tauri::command]
pub async fn compress_conversation(
    brain: State<'_, DesktopBrainService>,
    api_key: String,
    image_brief: String,
    history_to_compress: String,
) -> Result<String, String> {
    brain
        .compress_conversation(CompressConversationRequest {
            api_key,
            image_brief,
            history_to_compress,
        })
        .await
}

#[tauri::command]
pub async fn cancel_request(
    brain: State<'_, DesktopBrainService>,
    channel_id: Option<String>,
) -> Result<(), String> {
    brain.cancel_request(channel_id).await
}

#[tauri::command]
pub async fn quick_answer_request(
    brain: State<'_, DesktopBrainService>,
    channel_id: String,
) -> Result<(), String> {
    brain.quick_answer_request(channel_id).await
}
