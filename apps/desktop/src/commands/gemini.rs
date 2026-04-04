// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::{future::join_all, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
struct GeminiRequestControl {
    cancel_token: tokio_util::sync::CancellationToken,
    answer_now: Arc<AtomicBool>,
    answer_now_notify: Arc<tokio::sync::Notify>,
}

impl GeminiRequestControl {
    fn new() -> Self {
        Self {
            cancel_token: tokio_util::sync::CancellationToken::new(),
            answer_now: Arc::new(AtomicBool::new(false)),
            answer_now_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    fn request_answer_now(&self) {
        self.answer_now.store(true, Ordering::SeqCst);
        self.answer_now_notify.notify_waiters();
    }

    fn is_answer_now_requested(&self) -> bool {
        self.answer_now.load(Ordering::SeqCst)
    }
}

lazy_static::lazy_static! {
    static ref ACTIVE_REQUESTS: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, GeminiRequestControl>>> = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
}

#[tauri::command]
pub async fn cancel_gemini_request(channel_id: Option<String>) -> Result<(), String> {
    let mut map = ACTIVE_REQUESTS.lock().await;
    if let Some(id) = channel_id {
        log::info!("Cancelling request for channel: {}", id);
        if let Some(control) = map.remove(&id) {
            control.cancel_token.cancel();
        }
    } else {
        log::info!("Cancelling ALL Gemini requests");
        for (_, control) in map.drain() {
            control.cancel_token.cancel();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn answer_now_gemini_request(channel_id: String) -> Result<(), String> {
    let map = ACTIVE_REQUESTS.lock().await;
    if let Some(control) = map.get(&channel_id) {
        log::info!("Answer-now requested for channel: {}", channel_id);
        control.request_answer_now();
    } else {
        log::info!(
            "Answer-now requested for unknown channel (likely completed): {}",
            channel_id
        );
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
pub struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(rename = "toolConfig", skip_serializing_if = "Option::is_none")]
    tool_config: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseCandidate {
    content: Option<GeminiResponseContent>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
    #[serde(rename = "thoughtSignature")]
    thought_signature: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseChunk {
    candidates: Option<Vec<GeminiResponseCandidate>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GeminiEvent {
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

fn emit_event(app: &AppHandle, channel_id: &str, event: GeminiEvent) {
    let _ = app.emit(channel_id, event);
}

struct StreamIterationResult {
    function_call: Option<GeminiFunctionCall>,
    function_call_thought_signature: Option<String>,
}

async fn stream_request_iteration(
    app: &AppHandle,
    client: &reqwest::Client,
    url: &str,
    request_body: &GeminiRequest,
    channel_id: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
) -> Result<StreamIterationResult, String> {
    let response_result = tokio::select! {
        res = client.post(url).json(request_body).send() => res.map_err(|e| format!("Failed to send request to Gemini: {}", e)),
        _ = cancel_token.cancelled() => Err("CANCELLED".to_string()),
    };
    let response = response_result?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API Error: {}", error_text));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();
    let mut function_call: Option<GeminiFunctionCall> = None;
    let mut function_call_thought_signature: Option<String> = None;

    'stream_loop: loop {
        tokio::select! {
            chunk_opt = stream.next() => {
                match chunk_opt {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(idx) = buffer.find('\n') {
                            let line = buffer[..idx].to_string();
                            buffer.drain(..idx + 1);

                            let trimmed = line.trim();
                            if let Some(data) = trimmed.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    break 'stream_loop;
                                }
                                let Ok(chunk_data) = serde_json::from_str::<GeminiResponseChunk>(data) else {
                                    continue;
                                };
                                let Some(candidates) = chunk_data.candidates else {
                                    continue;
                                };
                                let Some(first) = candidates.first() else {
                                    continue;
                                };

                                if let Some(content) = &first.content {
                                    if let Some(parts) = &content.parts {
                                        for part in parts {
                                            if let Some(fc) = &part.function_call {
                                                if function_call.is_none() {
                                                    function_call = Some(fc.clone());
                                                    function_call_thought_signature =
                                                        part.thought_signature.clone();
                                                    if !full_text.is_empty() {
                                                        full_text.clear();
                                                        emit_event(app, channel_id, GeminiEvent::Reset);
                                                    }
                                                }
                                                continue;
                                            }
                                            if function_call.is_none() {
                                                if let Some(text) = &part.text {
                                                    full_text.push_str(text);
                                                    emit_event(
                                                        app,
                                                        channel_id,
                                                        GeminiEvent::Token {
                                                            token: text.clone(),
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }

                                if first.finish_reason.is_some() {
                                    break 'stream_loop;
                                }
                            }
                        }
                    }
                    Some(Err(e)) => return Err(format!("Stream error: {}", e)),
                    None => break,
                }
            }
            _ = cancel_token.cancelled() => return Err("CANCELLED".to_string()),
        }
    }

    Ok(StreamIterationResult {
        function_call,
        function_call_thought_signature,
    })
}

fn tool_step_id(iter: usize) -> String {
    format!("web-search-call-{}", iter + 1)
}

fn tool_status_text(function_call: &GeminiFunctionCall) -> Option<String> {
    let query = function_call
        .args
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let url = function_call
        .args
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());

    if query.is_some() {
        return Some("Searching for relevant sources".to_string());
    }
    if let Some(u) = url {
        return Some(format!("Fetching {}", u));
    }
    None
}

fn build_system_instruction_with_search_policy(
    user_name: &str,
    user_email: &str,
    image_brief: &str,
    tools_enabled: bool,
) -> Result<String, String> {
    let mut instruction =
        crate::brain::processor::build_system_instruction(user_name, user_email, image_brief)?;

    if tools_enabled {
        instruction.push_str(
            "\n\n## Web Search Policy\n\
             - If the user asks for current, time-sensitive, or uncertain facts, call `web_search`.\n\
             - If greeting/chit-chat, do not call tools.\n\
             - Never invent URLs or sources.\n\
             - When using `url`, only fetch URLs from prior search results in this turn.\n\
             - If one search pass is too shallow, call `web_search` again with a refined query.\n\
             - If web search fails repeatedly, answer from model knowledge and clearly state web search was unavailable.",
        );
    }

    Ok(instruction)
}

fn extract_text_from_non_stream_chunk(chunk: &GeminiResponseChunk) -> Option<String> {
    chunk.candidates.as_ref().and_then(|candidates| {
        candidates.first().and_then(|first| {
            first.content.as_ref().and_then(|content| {
                content.parts.as_ref().and_then(|parts| {
                    parts
                        .iter()
                        .find_map(|part| part.text.as_ref().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                })
            })
        })
    })
}

fn parse_url_array_from_text(raw: &str, max_urls: usize) -> Vec<String> {
    fn normalize_urls(values: Vec<String>, max_urls: usize) -> Vec<String> {
        let mut out = Vec::<String>::new();
        let mut seen = HashSet::<String>::new();
        for value in values {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(parsed) = url::Url::parse(trimmed) else {
                continue;
            };
            let scheme = parsed.scheme().to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                continue;
            }
            let normalized = parsed.to_string();
            if seen.insert(normalized.clone()) {
                out.push(normalized);
                if out.len() >= max_urls {
                    break;
                }
            }
        }
        out
    }

    let text = raw.trim();
    if text.is_empty() {
        return Vec::new();
    }

    if let Ok(values) = serde_json::from_str::<Vec<String>>(text) {
        return normalize_urls(values, max_urls);
    }

    let start = text.find('[');
    let end = text.rfind(']');
    if let (Some(s), Some(e)) = (start, end) {
        if s < e {
            let slice = &text[s..=e];
            if let Ok(values) = serde_json::from_str::<Vec<String>>(slice) {
                return normalize_urls(values, max_urls);
            }
        }
    }

    Vec::new()
}

async fn suggest_fallback_urls(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    query: &str,
    max_urls: usize,
) -> Vec<String> {
    let suggest_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let prompt = format!(
        "Suggest up to {max_urls} direct, publicly accessible URLs for this query: \"{query}\".\n\
         Prefer captcha-free sources like Wikipedia, official docs, blogs, and trusted public pages.\n\
         Return ONLY a JSON array of URLs. No markdown, no explanation."
    );

    let request_body = GeminiRequest {
        system_instruction: None,
        contents: vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: Some(prompt),
                ..Default::default()
            }],
        }],
        tools: None,
        tool_config: None,
    };

    let response = match tokio::time::timeout(
        Duration::from_secs(8),
        client.post(&suggest_url).json(&request_body).send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(_)) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    if !response.status().is_success() {
        return Vec::new();
    }

    let body = match tokio::time::timeout(Duration::from_secs(8), response.text()).await {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    let chunk: GeminiResponseChunk = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let Some(text) = extract_text_from_non_stream_chunk(&chunk) else {
        return Vec::new();
    };

    parse_url_array_from_text(&text, max_urls)
}

fn merge_allowed_sources(
    allowed_sources: &mut HashMap<String, crate::services::web_search::CitationSource>,
    result: &crate::services::web_search::WebSearchResult,
) {
    for (url, source) in crate::services::web_search::collect_allowed_sources(result) {
        allowed_sources.insert(url, source);
    }
}

fn track_attempted_sources(
    sources: &[crate::services::web_search::CitationSource],
    attempted_urls: &mut HashSet<String>,
    attempted_domains: &mut HashSet<String>,
) {
    for source in sources {
        attempted_urls.insert(source.url.clone());
        if let Some(domain) = crate::services::web_search::domain_from_url(&source.url) {
            attempted_domains.insert(domain);
        }
    }
}

fn mark_attempted_url(
    raw_url: &str,
    attempted_urls: &mut HashSet<String>,
    attempted_domains: &mut HashSet<String>,
) {
    attempted_urls.insert(raw_url.to_string());
    if let Some(domain) = crate::services::web_search::domain_from_url(raw_url) {
        attempted_domains.insert(domain);
    }
}

fn wrap_query_fallback_result(
    query: &str,
    mut result: crate::services::web_search::WebSearchResult,
    fallback_message: &str,
) -> crate::services::web_search::WebSearchResult {
    result.mode = "query".to_string();
    result.query = Some(query.trim().to_string());
    result.requested_url = None;
    if result.message.is_none() {
        result.message = Some(fallback_message.to_string());
    }
    result
}

enum ControlledAwaitOutcome<T> {
    Completed(T),
    Cancelled,
    AnswerNow,
}

async fn await_with_request_control<T>(
    future: impl std::future::Future<Output = T>,
    control: &GeminiRequestControl,
) -> ControlledAwaitOutcome<T> {
    if control.cancel_token.is_cancelled() {
        return ControlledAwaitOutcome::Cancelled;
    }
    if control.is_answer_now_requested() {
        return ControlledAwaitOutcome::AnswerNow;
    }

    tokio::select! {
        output = future => ControlledAwaitOutcome::Completed(output),
        _ = control.cancel_token.cancelled() => ControlledAwaitOutcome::Cancelled,
        _ = control.answer_now_notify.notified() => ControlledAwaitOutcome::AnswerNow,
    }
}

fn collect_answer_now_sources(
    allowed_sources: &HashMap<String, crate::services::web_search::CitationSource>,
    max_sources: usize,
) -> Vec<crate::services::web_search::CitationSource> {
    let mut sources = allowed_sources.values().cloned().collect::<Vec<_>>();
    sources.sort_by(|a, b| a.url.cmp(&b.url));
    sources.truncate(max_sources);
    sources
}

fn build_answer_now_context_markdown(
    mode_label: &str,
    sources: &[crate::services::web_search::CitationSource],
) -> String {
    if sources.is_empty() {
        return format!(
            "[{mode_label} interrupted by Answer Now]\n- No web sources were collected yet."
        );
    }

    let mut context = format!("[{mode_label} interrupted by Answer Now]\n");
    for source in sources {
        context.push_str(&format!(
            "- {} — {}\n  {}\n",
            source.title,
            source.url,
            if source.summary.trim().is_empty() {
                "(No snippet available)"
            } else {
                source.summary.trim()
            }
        ));
    }
    context.trim().to_string()
}

fn build_answer_now_partial_result(
    query: Option<&str>,
    requested_url: Option<&str>,
    allowed_sources: &HashMap<String, crate::services::web_search::CitationSource>,
) -> crate::services::web_search::WebSearchResult {
    let mode = if requested_url.is_some() {
        "url"
    } else {
        "query"
    };
    let sources = collect_answer_now_sources(allowed_sources, 6);
    crate::services::web_search::WebSearchResult {
        mode: mode.to_string(),
        query: query.map(|v| v.to_string()),
        requested_url: requested_url.map(|v| v.to_string()),
        context_markdown: build_answer_now_context_markdown("Search", &sources),
        sources,
        success: true,
        message: Some(
            "Answer requested before search completed; returning collected sources so far."
                .to_string(),
        ),
    }
}

/// Brain-aware chat command (v2)
///
/// For initial turns (is_initial_turn=true):
///   - Uses soul.yml + scenes.json to build system prompt (user content)
///   - Requires image_path
///   - user_instruction appended as one-time intent hook
///
/// For subsequent turns (is_initial_turn=false):
///   - Uses frame.md template with context anchors
///   - Requires image_description, user_first_msg, history_log
///
/// On ALL turns:
///   - system.yml is sent via native system_instruction field
///   - Contains: identity brief + OS + timezone + user profile + image_brief
#[tauri::command]
#[allow(clippy::too_many_arguments)]
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
    // Rolling summary of compressed older turns
    rolling_summary: Option<String>,
    // Current user message (empty on first turn for image-only analysis)
    user_message: String,
    channel_id: String,
    // Runtime context params (NEW)
    user_name: Option<String>,
    user_email: Option<String>,
    user_instruction: Option<String>,
    image_brief: Option<String>,
) -> Result<(), String> {
    const MAX_TOOL_CALLS_PER_TURN: usize = 3;
    const MAX_AGENT_ITERATIONS: usize = 8;

    let result = async {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );

        let request_control = GeminiRequestControl::new();
        {
            let mut map = ACTIVE_REQUESTS.lock().await;
            map.insert(channel_id.clone(), request_control.clone());
        }

        let mut allow_tools = !is_initial_turn;
        let mut tool_calls = 0usize;
        let mut consecutive_tool_failures = 0usize;
        let web_search_tool_declaration = if allow_tools {
            Some(crate::brain::loader::load_web_search_tool_declaration()?)
        } else {
            None
        };
        let mut allowed_sources =
            HashMap::<String, crate::services::web_search::CitationSource>::new();
        let mut attempted_urls = HashSet::<String>::new();
        let mut attempted_domains = HashSet::<String>::new();

        // Build conversation contents once; then append tool call/response turns as needed.
        let mut contents: Vec<GeminiContent> = if is_initial_turn {
            let system_prompt = crate::brain::processor::build_initial_system_prompt()?;
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

            if let Some(ref instruction) = user_instruction {
                if !instruction.trim().is_empty() {
                    parts.push(GeminiPart {
                        text: Some(format!("\n## User's Default Instruction\n{}", instruction)),
                        ..Default::default()
                    });
                }
            }

            if !user_message.is_empty() {
                let interleaved_parts =
                    build_interleaved_parts(&user_message, &api_key, &state.gemini_file_cache)
                        .await?;
                parts.extend(interleaved_parts);
            }

            vec![GeminiContent {
                role: "user".to_string(),
                parts,
            }]
        } else {
            let img_desc =
                image_description.ok_or("image_description required for subsequent turns")?;
            let first_msg = user_first_msg.unwrap_or_default();
            let history = history_log.unwrap_or_default();
            let summary = rolling_summary.clone().unwrap_or_default();
            let context_prompt =
                crate::brain::processor::build_turn_context(&img_desc, &first_msg, &history, &summary);

            let mut parts = vec![GeminiPart {
                text: Some(context_prompt),
                ..Default::default()
            }];
            let interleaved_parts =
                build_interleaved_parts(&user_message, &api_key, &state.gemini_file_cache).await?;
            parts.extend(interleaved_parts);
            vec![GeminiContent {
                role: "user".to_string(),
                parts,
            }]
        };

        for iter in 0..MAX_AGENT_ITERATIONS {
            if allow_tools && request_control.is_answer_now_requested() {
                allow_tools = false;
                emit_event(
                    &app,
                    &channel_id,
                    GeminiEvent::ToolStatus {
                        message: "Wrapping up with what I have so far".to_string(),
                    },
                );
            }

            let tools = if allow_tools {
                Some(vec![web_search_tool_declaration
                    .as_ref()
                    .ok_or_else(|| "Web search tool declaration not loaded".to_string())?
                    .clone()])
            } else {
                None
            };

            let sys_instruction = build_system_instruction_with_search_policy(
                user_name.as_deref().unwrap_or(""),
                user_email.as_deref().unwrap_or(""),
                image_brief.as_deref().unwrap_or(""),
                allow_tools,
            )?;
            let system_instruction = Some(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: Some(sys_instruction),
                    ..Default::default()
                }],
            });

            let request_body = GeminiRequest {
                system_instruction,
                contents: contents.clone(),
                tools,
                tool_config: if allow_tools {
                    Some(json!({
                        "functionCallingConfig": {
                            "mode": "AUTO"
                        }
                    }))
                } else {
                    None
                },
            };

            let iteration = stream_request_iteration(
                &app,
                &client,
                &url,
                &request_body,
                &channel_id,
                &request_control.cancel_token,
            )
            .await?;

            if !allow_tools {
                return Ok(());
            }

            let Some(function_call) = iteration.function_call else {
                return Ok(());
            };

            if function_call.name != "web_search" {
                allow_tools = false;
                continue;
            }

            let status_text = tool_status_text(&function_call);
            let call_id = tool_step_id(iter);
            if let Some(status_text_value) = status_text.as_ref() {
                emit_event(
                    &app,
                    &channel_id,
                    GeminiEvent::ToolStatus {
                        message: status_text_value.clone(),
                    },
                );
            }
            emit_event(
                &app,
                &channel_id,
                GeminiEvent::ToolStart {
                    id: call_id.clone(),
                    name: "web_search".to_string(),
                    args: function_call.args.clone(),
                    message: status_text.unwrap_or_default(),
                },
            );

            let query = function_call
                .args
                .get("query")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let requested_url = function_call
                .args
                .get("url")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());

            let tool_result = if request_control.is_answer_now_requested() {
                Ok(build_answer_now_partial_result(
                    query,
                    requested_url,
                    &allowed_sources,
                ))
            } else if let Some(q) = query {
                match await_with_request_control(
                    crate::services::web_search::search_query_with_progress(
                        q,
                        Some(6),
                        |message| {
                            emit_event(
                                &app,
                                &channel_id,
                                GeminiEvent::ToolStatus { message },
                            );
                        },
                    ),
                    &request_control,
                )
                .await
                {
                    ControlledAwaitOutcome::Completed(Ok(result)) => Ok(result),
                    ControlledAwaitOutcome::Completed(Err(search_error)) => {
                        println!("[WebSearch] Primary query failed: {}", search_error);
                        let mut final_error = search_error;
                        let mut resolved: Option<crate::services::web_search::WebSearchResult> =
                            None;

                        emit_event(
                            &app,
                            &channel_id,
                            GeminiEvent::ToolStatus {
                                message: "Trying another reliable source".to_string(),
                            },
                        );

                        let local_candidates =
                            crate::services::web_search::local_safe_source_candidates(
                                q,
                                &attempted_domains,
                                3,
                            );
                        println!(
                            "[WebSearch] Local safe-source candidates: {}",
                            local_candidates.len()
                        );

                        for candidate in local_candidates {
                            if attempted_urls.contains(&candidate.url) {
                                continue;
                            }

                            mark_attempted_url(
                                &candidate.url,
                                &mut attempted_urls,
                                &mut attempted_domains,
                            );
                            emit_event(
                                &app,
                                &channel_id,
                                GeminiEvent::ToolStatus {
                                    message: "Trying another reliable source".to_string(),
                                },
                            );

                            let mut allowed = HashMap::new();
                            allowed.insert(candidate.url.clone(), candidate.clone());

                            match await_with_request_control(
                                crate::services::web_search::fetch_url_from_allowed_with_progress(
                                    &candidate.url,
                                    &allowed,
                                    |message| {
                                        emit_event(
                                            &app,
                                            &channel_id,
                                            GeminiEvent::ToolStatus { message },
                                        );
                                    },
                                ),
                                &request_control,
                            )
                            .await
                            {
                                ControlledAwaitOutcome::Completed(Ok(result)) => {
                                    let fallback_message = format!(
                                        "Primary search failed; used trusted fallback source: {}.",
                                        candidate.title
                                    );
                                    resolved = Some(wrap_query_fallback_result(
                                        q,
                                        result,
                                        &fallback_message,
                                    ));
                                    break;
                                }
                                ControlledAwaitOutcome::Completed(Err(fetch_error)) => {
                                    println!(
                                        "[WebSearch] Local safe-source fallback failed: {}",
                                        fetch_error
                                    );
                                    final_error = fetch_error;
                                }
                                ControlledAwaitOutcome::Cancelled => {
                                    return Err("CANCELLED".to_string());
                                }
                                ControlledAwaitOutcome::AnswerNow => {
                                    resolved = Some(build_answer_now_partial_result(
                                        Some(q),
                                        None,
                                        &allowed_sources,
                                    ));
                                    break;
                                }
                            }
                        }

                        if resolved.is_none() {
                            emit_event(
                                &app,
                                &channel_id,
                                GeminiEvent::ToolStatus {
                                    message: "Trying another reliable source".to_string(),
                                },
                            );

                            let suggested_urls = match await_with_request_control(
                                suggest_fallback_urls(&client, &api_key, &model, q, 6),
                                &request_control,
                            )
                            .await
                            {
                                ControlledAwaitOutcome::Completed(urls) => urls,
                                ControlledAwaitOutcome::Cancelled => {
                                    return Err("CANCELLED".to_string());
                                }
                                ControlledAwaitOutcome::AnswerNow => {
                                    resolved = Some(build_answer_now_partial_result(
                                        Some(q),
                                        None,
                                        &allowed_sources,
                                    ));
                                    Vec::new()
                                }
                            };

                            if resolved.is_none() {
                                let filtered_candidates = crate::services::web_search::filter_suggested_urls_to_safe_sources(
                                    &suggested_urls,
                                    &attempted_domains,
                                    3,
                                );
                                println!(
                                    "[WebSearch] Gemini suggested {} URLs, {} passed safe filtering",
                                    suggested_urls.len(),
                                    filtered_candidates.len()
                                );

                                for candidate in filtered_candidates {
                                    if attempted_urls.contains(&candidate.url) {
                                        continue;
                                    }

                                    mark_attempted_url(
                                        &candidate.url,
                                        &mut attempted_urls,
                                        &mut attempted_domains,
                                    );
                                    emit_event(
                                        &app,
                                        &channel_id,
                                        GeminiEvent::ToolStatus {
                                            message: "Trying another reliable source".to_string(),
                                        },
                                    );

                                    let mut allowed = HashMap::new();
                                    allowed.insert(candidate.url.clone(), candidate.clone());

                                    match await_with_request_control(
                                        crate::services::web_search::fetch_url_from_allowed_with_progress(
                                            &candidate.url,
                                            &allowed,
                                            |message| {
                                                emit_event(
                                                    &app,
                                                    &channel_id,
                                                    GeminiEvent::ToolStatus { message },
                                                );
                                            },
                                        ),
                                        &request_control,
                                    )
                                    .await
                                    {
                                        ControlledAwaitOutcome::Completed(Ok(result)) => {
                                            let fallback_message = format!(
                                                "Primary search failed; used model-assisted trusted fallback source: {}.",
                                                candidate.title
                                            );
                                            resolved = Some(wrap_query_fallback_result(
                                                q,
                                                result,
                                                &fallback_message,
                                            ));
                                            break;
                                        }
                                        ControlledAwaitOutcome::Completed(Err(fetch_error)) => {
                                            println!(
                                                "[WebSearch] Gemini-assisted fallback failed: {}",
                                                fetch_error
                                            );
                                            final_error = fetch_error;
                                        }
                                        ControlledAwaitOutcome::Cancelled => {
                                            return Err("CANCELLED".to_string());
                                        }
                                        ControlledAwaitOutcome::AnswerNow => {
                                            resolved = Some(build_answer_now_partial_result(
                                                Some(q),
                                                None,
                                                &allowed_sources,
                                            ));
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        resolved.ok_or_else(|| {
                            format!(
                                "Search unavailable after all fallbacks. Last error: {}",
                                final_error
                            )
                        })
                    }
                    ControlledAwaitOutcome::Cancelled => return Err("CANCELLED".to_string()),
                    ControlledAwaitOutcome::AnswerNow => {
                        Ok(build_answer_now_partial_result(
                            Some(q),
                            None,
                            &allowed_sources,
                        ))
                    }
                }
            } else if let Some(u) = requested_url {
                mark_attempted_url(u, &mut attempted_urls, &mut attempted_domains);
                match await_with_request_control(
                    crate::services::web_search::fetch_url_from_allowed_with_progress(
                        u,
                        &allowed_sources,
                        |message| {
                            emit_event(
                                &app,
                                &channel_id,
                                GeminiEvent::ToolStatus { message },
                            );
                        },
                    ),
                    &request_control,
                )
                .await
                {
                    ControlledAwaitOutcome::Completed(result) => result,
                    ControlledAwaitOutcome::Cancelled => return Err("CANCELLED".to_string()),
                    ControlledAwaitOutcome::AnswerNow => {
                        Ok(build_answer_now_partial_result(
                            None,
                            Some(u),
                            &allowed_sources,
                        ))
                    }
                }
            } else {
                Err("Tool call requires either `query` or `url`.".to_string())
            };

            let (tool_response_value, tool_status, tool_message) = match tool_result {
                Ok(result) => {
                    merge_allowed_sources(&mut allowed_sources, &result);
                    track_attempted_sources(
                        &result.sources,
                        &mut attempted_urls,
                        &mut attempted_domains,
                    );
                    consecutive_tool_failures = 0;
                    let done_message = result
                        .message
                        .clone()
                        .unwrap_or_else(|| "Web search step completed.".to_string());
                    (
                        serde_json::to_value(&result).unwrap_or_else(|_| {
                            json!({
                                "success": false,
                                "sources": [],
                                "message": "Serialization failure for tool output"
                            })
                        }),
                        "done".to_string(),
                        done_message,
                    )
                }
                Err(error_message) => {
                    consecutive_tool_failures += 1;
                    (
                        json!({
                            "mode": if requested_url.is_some() { "url" } else { "query" },
                            "success": false,
                            "sources": [],
                            "context_markdown": "",
                            "message": error_message
                        }),
                        "error".to_string(),
                        "Web search step failed.".to_string(),
                    )
                }
            };

            emit_event(
                &app,
                &channel_id,
                GeminiEvent::ToolEnd {
                    id: call_id,
                    name: "web_search".to_string(),
                    status: tool_status,
                    result: tool_response_value.clone(),
                    message: tool_message,
                },
            );

            if request_control.is_answer_now_requested() {
                allow_tools = false;
                emit_event(
                    &app,
                    &channel_id,
                    GeminiEvent::ToolStatus {
                        message: "Wrapping up with what I have so far".to_string(),
                    },
                );
            }

            tool_calls += 1;
            if tool_calls >= MAX_TOOL_CALLS_PER_TURN {
                allow_tools = false;
                emit_event(
                    &app,
                    &channel_id,
                    GeminiEvent::ToolStatus {
                        message: "Wrapping up with what I have so far".to_string(),
                    },
                );
            }
            if consecutive_tool_failures >= 2 {
                allow_tools = false;
                emit_event(
                    &app,
                    &channel_id,
                    GeminiEvent::ToolStatus {
                        message:
                            "Search is unavailable right now, continuing with available context"
                                .to_string(),
                    },
                );
            }

            contents.push(GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    function_call: Some(function_call.clone()),
                    thought_signature: iteration.function_call_thought_signature.clone(),
                    ..Default::default()
                }],
            });
            contents.push(GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    function_response: Some(GeminiFunctionResponse {
                        name: function_call.name.clone(),
                        response: tool_response_value,
                    }),
                    ..Default::default()
                }],
            });
        }

        Err("Maximum tool iterations reached without final response.".to_string())
    }
    .await;

    let mut map = ACTIVE_REQUESTS.lock().await;
    map.remove(&channel_id);

    result
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
    let file_ref = crate::commands::gemini_files::ensure_file_uploaded(
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
