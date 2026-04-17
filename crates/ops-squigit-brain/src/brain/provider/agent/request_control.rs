// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

pub async fn cancel_request(
    runtime: &crate::runtime::BrainRuntimeState,
    channel_id: Option<String>,
) -> Result<(), String> {
    crate::brain::provider::gemini::agent::request_control::cancel_gemini_request(
        runtime, channel_id,
    )
    .await
}

pub async fn quick_answer_request(
    runtime: &crate::runtime::BrainRuntimeState,
    channel_id: String,
) -> Result<(), String> {
    crate::brain::provider::gemini::agent::request_control::answer_now_gemini_request(
        runtime, channel_id,
    )
    .await
}
