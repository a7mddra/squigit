// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

lazy_static::lazy_static! {
    pub static ref ACTIVE_REQUESTS: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>> = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
}

#[tauri::command]
pub async fn cancel_gemini_request(channel_id: String) -> Result<(), String> {
    log::info!("Cancelling request for channel: {}", channel_id);
    let mut map = ACTIVE_REQUESTS.lock().await;
    if let Some(token) = map.remove(&channel_id) {
        token.cancel();
    }
    Ok(())
}

async fn build_interleaved_parts(
    text: &str,
    api_key: &str,
    cache: &std::sync::Arc<
        tokio::sync::Mutex<
            std::collections::HashMap<String, crate::commands::gemini_files::GeminiFileRef>,
        >,
    >,
) -> Result<Vec<GeminiPart>, String> {
    enum PreparedAttachment {
        Uploaded(crate::commands::gemini_files::GeminiFileRef),
        InlineText(String),
    }

    let re = Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("Regex Error: {}", e))?;

    let mut text_chunks = Vec::new();
    let mut last_end = 0;
    let mut file_paths = Vec::new();

    for cap in re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let path = cap.get(1).unwrap().as_str().to_string();

        let before = &text[last_end..full_match.start()];
        if !before.trim().is_empty() {
            text_chunks.push((false, before.to_string()));
        }

        file_paths.push(path.clone());
        text_chunks.push((true, path));

        last_end = full_match.end();
    }

    let remaining = &text[last_end..];
    if !remaining.trim().is_empty() {
        text_chunks.push((false, remaining.to_string()));
    }

    if file_paths.is_empty() {
        return Ok(vec![GeminiPart {
            text: Some(text.to_string()),
            ..Default::default()
        }]);
    }

    let mut unique_paths = file_paths.clone();
    unique_paths.sort();
    unique_paths.dedup();

    let prepare_futures = unique_paths.iter().map(|p| async {
        if crate::commands::gemini_files::is_docx_path(p) {
            let extracted_text =
                crate::commands::gemini_files::extract_docx_text_for_prompt(p).await?;
            Ok::<PreparedAttachment, String>(PreparedAttachment::InlineText(extracted_text))
        } else {
            let file_ref =
                crate::commands::gemini_files::ensure_file_uploaded(api_key, p, cache).await?;
            Ok::<PreparedAttachment, String>(PreparedAttachment::Uploaded(file_ref))
        }
    });

    let results = join_all(prepare_futures).await;
    let mut prepared_attachments = std::collections::HashMap::new();

    for (path, result) in unique_paths.into_iter().zip(results.into_iter()) {
        match result {
            Ok(prepared) => {
                prepared_attachments.insert(path, prepared);
            }
            Err(e) => return Err(e),
        }
    }

    let mut parts = Vec::new();
    for (is_file, content) in text_chunks {
        if is_file {
            if let Some(prepared) = prepared_attachments.get(&content) {
                match prepared {
                    PreparedAttachment::Uploaded(file_ref) => {
                        parts.push(GeminiPart {
                            file_data: Some(GeminiFileData {
                                mime_type: file_ref.mime_type.clone(),
                                file_uri: file_ref.file_uri.clone(),
                            }),
                            ..Default::default()
                        });
                    }
                    PreparedAttachment::InlineText(extracted_text) => {
                        let file_name = std::path::Path::new(&content)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("attachment.docx");
                        let docx_block = format!(
                            "[Attachment: {} | format: docx | content: extracted text]\n{}\n[End attachment]",
                            file_name, extracted_text
                        );
                        parts.push(GeminiPart {
                            text: Some(docx_block),
                            ..Default::default()
                        });
                    }
                }
            }
        } else {
            parts.push(GeminiPart {
                text: Some(content),
                ..Default::default()
            });
        }
    }

    Ok(parts)
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "fileData", skip_serializing_if = "Option::is_none")]
    pub file_data: Option<GeminiFileData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFileData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "fileUri")]
    pub file_uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseCandidate {
    content: Option<GeminiResponseContent>,
    #[serde(rename = "finishReason")]
    _finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseChunk {
    candidates: Option<Vec<GeminiResponseCandidate>>,
}

#[derive(Debug, Serialize, Clone)]
struct GeminiEvent {
    token: String,
}

/// Brain-aware chat command (v2)
///
/// For initial turns (is_initial_turn=true):
///   - Uses soul.yml + scenes.json to build system prompt
///   - Requires image_base64 and image_mime_type
///
/// For subsequent turns (is_initial_turn=false):
///   - Uses frame.md template with context anchors
///   - Requires image_description, user_first_msg, history_log
#[tauri::command]
pub async fn stream_gemini_chat_v2(
    app: AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
    api_key: String,
    model: String,
    is_initial_turn: bool,
    // Initial turn params
    image_path: Option<String>,
    // Subsequent turn params
    image_description: Option<String>,
    user_first_msg: Option<String>,
    history_log: Option<String>,
    // Current user message (empty on first turn for image-only analysis)
    user_message: String,
    channel_id: String,
) -> Result<(), String> {
    use crate::brain::processor::{build_initial_system_prompt, build_turn_context};

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
        model, api_key
    );

    // Build contents based on turn type
    let contents: Vec<GeminiContent> = if is_initial_turn {
        // Initial turn: soul + scenes + image
        let system_prompt = build_initial_system_prompt()?;

        let mut parts = vec![];

        if let Some(path) = image_path.clone() {
            let file_ref = crate::commands::gemini_files::ensure_file_uploaded(
                &api_key,
                &path,
                &state.gemini_file_cache,
            )
            .await?;
            parts.push(GeminiPart {
                file_data: Some(GeminiFileData {
                    mime_type: file_ref.mime_type.clone(),
                    file_uri: file_ref.file_uri.clone(),
                }),
                ..Default::default()
            });
        } else {
            return Err("image_path required for initial turn".to_string());
        }

        parts.push(GeminiPart {
            text: Some(system_prompt),
            ..Default::default()
        });

        // Add user message if provided
        if !user_message.is_empty() {
            let interleaved_parts =
                build_interleaved_parts(&user_message, &api_key, &state.gemini_file_cache).await?;
            parts.extend(interleaved_parts);
        }

        vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }]
    } else {
        // Subsequent turn: frame.md with context
        let img_desc =
            image_description.ok_or("image_description required for subsequent turns")?;
        let first_msg = user_first_msg.unwrap_or_default();
        let history = history_log.unwrap_or_default();

        let context_prompt = build_turn_context(&img_desc, &first_msg, &history);

        let mut parts = vec![GeminiPart {
            text: Some(context_prompt),
            ..Default::default()
        }];

        // Re-send image if provided (e.g. for first user intent message)
        if let Some(path) = image_path {
            let file_ref = crate::commands::gemini_files::ensure_file_uploaded(
                &api_key,
                &path,
                &state.gemini_file_cache,
            )
            .await?;
            parts.push(GeminiPart {
                file_data: Some(GeminiFileData {
                    mime_type: file_ref.mime_type.clone(),
                    file_uri: file_ref.file_uri.clone(),
                }),
                ..Default::default()
            });
        }

        let interleaved_parts =
            build_interleaved_parts(&user_message, &api_key, &state.gemini_file_cache).await?;
        parts.extend(interleaved_parts);

        vec![GeminiContent {
            role: "user".to_string(),
            parts,
        }]
    };

    let request_body = GeminiRequest { contents };

    let cancel_token = tokio_util::sync::CancellationToken::new();
    {
        let mut map = ACTIVE_REQUESTS.lock().await;
        map.insert(channel_id.clone(), cancel_token.clone());
    }

    let req_future = client.post(&url).json(&request_body).send();

    let response_result = tokio::select! {
        res = req_future => res,
        _ = cancel_token.cancelled() => {
            let mut map = ACTIVE_REQUESTS.lock().await;
            map.remove(&channel_id);
            return Err("CANCELLED".to_string());
        }
    };

    {
        let mut map = ACTIVE_REQUESTS.lock().await;
        map.remove(&channel_id);
    }

    let response =
        response_result.map_err(|e| format!("Failed to send request to Gemini: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API Error: {}", error_text));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let chunks: Vec<GeminiResponseChunk> = serde_json::from_str(&body).map_err(|e| {
        format!(
            "Failed to parse Gemini response: {} - Body: {}",
            e,
            &body[..body.len().min(500)]
        )
    })?;

    for chunk in chunks {
        if let Some(candidates) = chunk.candidates {
            if let Some(first) = candidates.first() {
                if let Some(content) = &first.content {
                    if let Some(parts) = &content.parts {
                        for part in parts {
                            if let Some(text) = &part.text {
                                let _ = app.emit(
                                    &channel_id,
                                    GeminiEvent {
                                        token: text.clone(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
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

    let request_body = GeminiRequest { contents };

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
    Ok("New Chat".to_string())
}
