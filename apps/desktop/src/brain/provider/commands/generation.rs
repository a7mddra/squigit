// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

#[tauri::command]
pub async fn generate_chat_title(
    api_key: String,
    model: String,
    prompt_context: String,
) -> Result<String, String> {
    crate::brain::provider::gemini::commands::generation::generate_chat_title(
        api_key,
        model,
        prompt_context,
    )
    .await
}

#[tauri::command]
pub async fn generate_image_brief(
    state: tauri::State<'_, crate::state::AppState>,
    api_key: String,
    image_path: String,
    model: Option<String>,
) -> Result<String, String> {
    crate::brain::provider::gemini::commands::generation::generate_image_brief(
        state, api_key, image_path, model,
    )
    .await
}

#[tauri::command]
pub async fn compress_conversation(
    api_key: String,
    image_brief: String,
    history_to_compress: String,
) -> Result<String, String> {
    crate::brain::provider::gemini::commands::generation::compress_conversation(
        api_key,
        image_brief,
        history_to_compress,
    )
    .await
}
