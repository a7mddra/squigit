// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

#[allow(clippy::too_many_arguments)]
pub async fn stream_chat(
    runtime: &crate::runtime::BrainRuntimeState,
    sink: &dyn crate::events::BrainEventSink,
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
    crate::brain::provider::gemini::commands::chat::stream_gemini_chat_v2(
        runtime,
        sink,
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
    )
    .await
}
