// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::brain::provider::gemini::transport::types::{
    GeminiContent, GeminiFileData, GeminiPart, GeminiRequest, GeminiResponseChunk,
};

/// Generate a chat title for the chat using the brain's title prompt and the text context.
/// Returns the generated title text directly.
pub async fn generate_chat_title(
    api_key: String,
    model: String,
    prompt_context: String,
) -> Result<String, String> {
    use crate::brain::context::builder::get_title_prompt;
    println!("Generating Title using model: {}", model);

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let title_context: String = prompt_context
        .lines()
        .take(3)
        .collect::<Vec<&str>>()
        .join("\n");
    let title_prompt_base = get_title_prompt().map_err(|e| e.to_string())?;
    let title_prompt = format!("{}\n\nContext:\n{}", title_prompt_base, title_context);

    let parts = vec![GeminiPart {
        text: Some(title_prompt),
        ..Default::default()
    }];

    let contents = vec![GeminiContent {
        role: "user".to_string(),
        parts,
    }];

    let request_body = GeminiRequest {
        system_instruction: None,
        contents,
        generation_config: None,
        tools: None,
        tool_config: None,
    };

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Gemini: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("Title Gen Error Status: {}", error_text);
        return Err(format!("Gemini API Error: {}", error_text));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    println!("Title Gen Success Body: {}", body);

    // Parse single response
    let chunk: GeminiResponseChunk = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Failed to parse Gemini response: {} - Body: {}",
            e,
            &body[..body.len().min(500)]
        )
    })?;

    // Extract text from response
    if let Some(candidates) = chunk.candidates {
        if let Some(first) = candidates.first() {
            if let Some(content) = &first.content {
                if let Some(parts) = &content.parts {
                    for part in parts {
                        if let Some(text) = &part.text {
                            println!("Title Generated: {}", text);
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    println!("Title Gen Failed to extract text from candidates");
    Ok("New thread".to_string())
}

/// Generate a lightweight text description of an image using the cheapest model.
/// Returns a 2-3 sentence plain-text description of what the image shows.
/// This runs in parallel with the main analysis and the result is stored
/// as `image_brief` in system_instruction for all subsequent turns.
pub async fn generate_image_brief(
    state: tauri::State<'_, crate::state::AppState>,
    api_key: String,
    image_path: String,
    model: Option<String>,
) -> Result<String, String> {
    use crate::brain::context::builder::get_image_brief_prompt;

    let brief_prompt = get_image_brief_prompt()?;
    let lite_model = model.unwrap_or_else(|| crate::constants::DEFAULT_MODEL.to_string());

    println!("[ImageBrief] Generating brief using model: {}", lite_model);

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        lite_model, api_key
    );

    // Upload image via Files API (reuses cache)
    let file_ref = crate::brain::provider::gemini::attachments::ensure_file_uploaded(
        &api_key,
        &image_path,
        &state.provider_file_cache,
    )
    .await?;

    let parts = vec![
        GeminiPart {
            file_data: Some(GeminiFileData {
                mime_type: file_ref.mime_type.clone(),
                file_uri: file_ref.file_uri.clone(),
            }),
            ..Default::default()
        },
        GeminiPart {
            text: Some(brief_prompt),
            ..Default::default()
        },
    ];

    let contents = vec![GeminiContent {
        role: "user".to_string(),
        parts,
    }];

    let request_body = GeminiRequest {
        system_instruction: None,
        contents,
        generation_config: None,
        tools: None,
        tool_config: None,
    };

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send image brief request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("[ImageBrief] Error: {}", error_text);
        return Err(format!("Image brief API error: {}", error_text));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read image brief response: {}", e))?;

    let chunk: GeminiResponseChunk = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Failed to parse image brief response: {} - Body: {}",
            e,
            &body[..body.len().min(500)]
        )
    })?;

    if let Some(candidates) = chunk.candidates {
        if let Some(first) = candidates.first() {
            if let Some(content) = &first.content {
                if let Some(parts) = &content.parts {
                    for part in parts {
                        if let Some(text) = &part.text {
                            println!("[ImageBrief] Generated: {}", text.trim());
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    println!("[ImageBrief] Failed to extract text, returning empty");
    Ok(String::new())
}

/// Compress older conversation turns into a rolling summary.
/// Uses the cheapest/fastest model (same as image_brief and title gen).
/// Non-streaming, returns the compressed summary text.
pub async fn compress_conversation(
    api_key: String,
    image_brief: String,
    history_to_compress: String,
) -> Result<String, String> {
    let summary_prompt =
        crate::brain::context::compactor::build_summary_prompt(&image_brief, &history_to_compress);
    let lite_model = crate::constants::DEFAULT_MODEL;

    println!(
        "[Summarizer] Compressing conversation using model: {}",
        lite_model
    );

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        lite_model, api_key
    );

    let parts = vec![GeminiPart {
        text: Some(summary_prompt),
        ..Default::default()
    }];

    let contents = vec![GeminiContent {
        role: "user".to_string(),
        parts,
    }];

    let request_body = GeminiRequest {
        system_instruction: None,
        contents,
        generation_config: None,
        tools: None,
        tool_config: None,
    };

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send compress request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("[Summarizer] Error: {}", error_text);
        return Err(format!("Compress API error: {}", error_text));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read compress response: {}", e))?;

    let chunk: GeminiResponseChunk = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Failed to parse compress response: {} - Body: {}",
            e,
            &body[..body.len().min(500)]
        )
    })?;

    if let Some(candidates) = chunk.candidates {
        if let Some(first) = candidates.first() {
            if let Some(content) = &first.content {
                if let Some(parts) = &content.parts {
                    for part in parts {
                        if let Some(text) = &part.text {
                            println!("[Summarizer] Compressed to {} chars", text.trim().len());
                            return Ok(text.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    println!("[Summarizer] Failed to extract summary, returning empty");
    Ok(String::new())
}
