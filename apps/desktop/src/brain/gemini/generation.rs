// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use super::types::{GeminiContent, GeminiFileData, GeminiPart, GeminiRequest, GeminiResponseChunk};
use serde::Deserialize;

const MAX_ATTACHMENT_MEMORY_ITEMS: usize = 6;
const MAX_ATTACHMENT_TEXT_BYTES: usize = 24_000;
const MAX_ATTACHMENT_SNIPPET_CHARS: usize = 320;

#[derive(Debug, Deserialize)]
pub struct AttachmentMemoryInput {
    pub path: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

fn attachment_kind_from_extension(extension: &str) -> &'static str {
    match extension {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" => "image",
        "pdf" => "pdf",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf" => "document",
        "txt" | "md" | "csv" | "json" | "xml" | "yaml" | "yml" | "html" | "css" | "js" | "ts"
        | "jsx" | "tsx" | "py" | "rs" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "sql"
        | "toml" | "sh" | "bash" => "text",
        _ => "file",
    }
}

fn normalize_for_memory(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return String::new();
    }

    let total_chars = compact.chars().count();
    let mut trimmed = compact.chars().take(max_chars).collect::<String>();
    if total_chars > max_chars {
        trimmed.push_str("...");
    }
    trimmed
}

async fn read_text_attachment_snippet(path: &std::path::Path) -> Result<String, String> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("Failed to open text attachment: {}", e))?;

    let mut buffer = vec![0u8; MAX_ATTACHMENT_TEXT_BYTES];
    let bytes_read = file
        .read(&mut buffer)
        .await
        .map_err(|e| format!("Failed to read text attachment: {}", e))?;
    buffer.truncate(bytes_read);

    if buffer.is_empty() {
        return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn default_attachment_summary(kind: &str) -> &'static str {
    match kind {
        "image" => "Image attachment provided in this turn.",
        "pdf" => "PDF attachment provided in this turn.",
        "document" => "Document attachment provided in this turn.",
        _ => "Attachment provided in this turn.",
    }
}

/// Build a compact attachment-memory block for conversation history/windowing.
/// This runs locally (no extra model call) and helps preserve uploaded media context
/// after the immediate turn where files were attached.
#[tauri::command]
pub async fn build_attachment_memory_context(
    attachments: Vec<AttachmentMemoryInput>,
) -> Result<String, String> {
    use std::collections::{HashMap, HashSet};

    let mut unique_paths = Vec::<String>::new();
    let mut display_names = HashMap::<String, String>::new();
    let mut seen = HashSet::<String>::new();
    for attachment in attachments {
        let trimmed = attachment.path.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            unique_paths.push(trimmed.to_string());
        }
        if let Some(display_name) = attachment
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            display_names.insert(trimmed.to_string(), display_name.to_string());
        }
    }

    if unique_paths.is_empty() {
        return Ok(String::new());
    }

    let mut lines = Vec::<String>::new();

    for raw_path in unique_paths.into_iter().take(MAX_ATTACHMENT_MEMORY_ITEMS) {
        let fallback_name = std::path::Path::new(&raw_path)
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or(raw_path.as_str())
            .to_string();

        let resolved =
            match crate::brain::gemini::attachment_paths::resolve_attachment_path_internal(
                &raw_path,
            ) {
                Ok(path) => path,
                Err(_) => {
                    lines.push(format!(
                        "- `{}` (file): Attachment path could not be resolved.",
                        fallback_name
                    ));
                    continue;
                }
            };

        let file_name = display_names
            .get(&raw_path)
            .cloned()
            .unwrap_or_else(|| {
                resolved
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or(fallback_name.as_str())
                    .to_string()
            });
        let extension = resolved
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = attachment_kind_from_extension(&extension);

        let mut summary = if extension == "docx" {
            let resolved_path = resolved.to_string_lossy().to_string();
            match crate::brain::gemini::files::extract_docx_text_for_prompt(&resolved_path).await {
                Ok(text) => normalize_for_memory(&text, MAX_ATTACHMENT_SNIPPET_CHARS),
                Err(_) => String::new(),
            }
        } else if kind == "text" {
            match read_text_attachment_snippet(&resolved).await {
                Ok(text) => normalize_for_memory(&text, MAX_ATTACHMENT_SNIPPET_CHARS),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        if summary.is_empty() {
            summary = default_attachment_summary(kind).to_string();
        }

        lines.push(format!("- `{}` ({}): {}", file_name, kind, summary));
    }

    if lines.is_empty() {
        return Ok(String::new());
    }

    Ok(format!("[Attachment Context]\n{}", lines.join("\n")))
}

/// Generate a chat title for the chat using the brain's title prompt and the text context.
/// Returns the generated title text directly.
#[tauri::command]
pub async fn generate_chat_title(
    api_key: String,
    model: String,
    prompt_context: String,
) -> Result<String, String> {
    use crate::brain::processor::get_title_prompt;
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
#[tauri::command]
pub async fn generate_image_brief(
    state: tauri::State<'_, crate::state::AppState>,
    api_key: String,
    image_path: String,
) -> Result<String, String> {
    use crate::brain::processor::get_image_brief_prompt;

    let brief_prompt = get_image_brief_prompt()?;
    let lite_model = crate::constants::DEFAULT_MODEL;

    println!("[ImageBrief] Generating brief using model: {}", lite_model);

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        lite_model, api_key
    );

    // Upload image via Files API (reuses cache)
    let file_ref = crate::brain::gemini::files::ensure_file_uploaded(
        &api_key,
        &image_path,
        &state.gemini_file_cache,
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
#[tauri::command]
pub async fn compress_conversation(
    api_key: String,
    image_brief: String,
    history_to_compress: String,
) -> Result<String, String> {
    let summary_prompt =
        crate::brain::memory::build_summary_prompt(&image_brief, &history_to_compress);
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
