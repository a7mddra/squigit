// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde_json::json;
use std::collections::{HashMap, HashSet};

use super::request_control::GeminiRequestControl;
use super::tool_orchestrator::{
    await_with_request_control, build_answer_now_partial_result, mark_attempted_url,
    merge_allowed_sources, track_attempted_sources, wrap_query_fallback_result,
    ControlledAwaitOutcome,
};
use crate::provider::gemini::transport::types::{GeminiFunctionCall, GeminiPart};
use crate::tools::web::suggest_fallback_urls;

#[derive(Default)]
pub(crate) struct WebToolDispatchState {
    pub(crate) allowed_sources: HashMap<String, crate::tools::web::CitationSource>,
    pub(crate) attempted_urls: HashSet<String>,
    pub(crate) attempted_domains: HashSet<String>,
}

pub(crate) struct ToolDispatchContext<'a> {
    pub(crate) client: &'a reqwest::Client,
    pub(crate) api_key: &'a str,
    pub(crate) model: &'a str,
    pub(crate) chat_id: Option<&'a str>,
    pub(crate) gemini_file_cache: &'a std::sync::Arc<
        tokio::sync::Mutex<
            HashMap<String, crate::provider::gemini::attachments::GeminiFileRef>,
        >,
    >,
    pub(crate) request_control: &'a GeminiRequestControl,
    pub(crate) web_state: &'a mut WebToolDispatchState,
}

pub(crate) struct ToolDispatchResult {
    pub(crate) response_value: serde_json::Value,
    pub(crate) follow_up_parts: Vec<GeminiPart>,
    pub(crate) status: String,
    pub(crate) message: String,
    pub(crate) is_failure: bool,
}

pub(crate) async fn dispatch_tool_call<F>(
    function_call: &GeminiFunctionCall,
    context: &mut ToolDispatchContext<'_>,
    mut emit_status: F,
) -> Result<ToolDispatchResult, String>
where
    F: FnMut(String) + Send,
{
    match function_call.name.as_str() {
        "web_search" => execute_web_search(function_call, context, &mut emit_status).await,
        "read_local_attachment_context" => {
            Ok(execute_local_attachment_context(function_call, context).await)
        }
        "recall_chat_attachment" => {
            Ok(execute_recall_chat_attachment(function_call, context).await)
        }
        _ => Ok(ToolDispatchResult {
            response_value: json!({
                "ok": false,
                "error_code": "unknown_tool",
                "error_message": format!("Unsupported tool: {}", function_call.name)
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: format!("Unsupported tool call `{}`.", function_call.name),
            is_failure: true,
        }),
    }
}

async fn execute_local_attachment_context(
    function_call: &GeminiFunctionCall,
    context: &mut ToolDispatchContext<'_>,
) -> ToolDispatchResult {
    let path = function_call
        .args
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty());

    let Some(path) = path else {
        return ToolDispatchResult {
            response_value: json!({
                "ok": false,
                "path": "",
                "error_code": "invalid_arguments",
                "error_message": "Tool call requires `path`"
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: "Local attachment read failed: missing `path`.".to_string(),
            is_failure: true,
        };
    };

    let requested_max = function_call.args.get("max_chars").and_then(|value| {
        value
            .as_u64()
            .map(|v| v as usize)
            .or_else(|| value.as_i64().map(|v| v.max(1) as usize))
    });
    let bounded_max =
        crate::provider::gemini::attachments::clamp_tool_max_chars(requested_max);

    let result = crate::provider::gemini::attachments::read_local_attachment_context(
        path,
        Some(bounded_max),
    )
    .await;

    let (status, message, is_failure) = match &result {
        crate::provider::gemini::attachments::LocalAttachmentContextResult::Success(
            success,
        ) => {
            let msg = if success.truncated {
                format!(
                    "Read local attachment context ({} chars, truncated).",
                    success.char_count
                )
            } else {
                format!(
                    "Read local attachment context ({} chars).",
                    success.char_count
                )
            };
            ("done".to_string(), msg, false)
        }
        crate::provider::gemini::attachments::LocalAttachmentContextResult::Failure(
            failure,
        ) => (
            "error".to_string(),
            format!("Local attachment read failed: {}", failure.error_message),
            true,
        ),
    };

    if context.request_control.is_answer_now_requested() {
        return ToolDispatchResult {
            response_value: result.to_json_value(),
            follow_up_parts: Vec::new(),
            status,
            message: "Answer requested while reading local context; returning current result."
                .to_string(),
            is_failure,
        };
    }

    ToolDispatchResult {
        response_value: result.to_json_value(),
        follow_up_parts: Vec::new(),
        status,
        message,
        is_failure,
    }
}

async fn execute_recall_chat_attachment(
    function_call: &GeminiFunctionCall,
    context: &mut ToolDispatchContext<'_>,
) -> ToolDispatchResult {
    let Some(chat_id) = context.chat_id else {
        return ToolDispatchResult {
            response_value: json!({
                "ok": false,
                "error_code": "missing_chat_context",
                "error_message": "Attachment recall requires an active chat session."
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: "Attachment recall failed: missing active chat context.".to_string(),
            is_failure: true,
        };
    };

    let target = function_call
        .args
        .get("target")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(target) = target else {
        return ToolDispatchResult {
            response_value: json!({
                "ok": false,
                "error_code": "invalid_arguments",
                "error_message": "Tool call requires `target`."
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: "Attachment recall failed: missing `target`.".to_string(),
            is_failure: true,
        };
    };

    let kind = function_call
        .args
        .get("kind")
        .and_then(|value| value.as_str());
    let reason = function_call
        .args
        .get("reason")
        .and_then(|value| value.as_str());

    match crate::provider::gemini::attachments::recall_chat_attachment(
        chat_id,
        target,
        kind,
        reason,
        context.api_key,
        context.gemini_file_cache,
    )
    .await
    {
        Ok(outcome) => ToolDispatchResult {
            response_value: outcome.response_value,
            follow_up_parts: outcome.follow_up_parts,
            status: if outcome.is_failure {
                "error".to_string()
            } else {
                "done".to_string()
            },
            message: outcome.message,
            is_failure: outcome.is_failure,
        },
        Err(error) => ToolDispatchResult {
            response_value: json!({
                "ok": false,
                "error_code": "recall_failed",
                "error_message": error,
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: "Attachment recall failed.".to_string(),
            is_failure: true,
        },
    }
}

async fn execute_web_search<F>(
    function_call: &GeminiFunctionCall,
    context: &mut ToolDispatchContext<'_>,
    emit_status: &mut F,
) -> Result<ToolDispatchResult, String>
where
    F: FnMut(String) + Send,
{
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

    let tool_result = if context.request_control.is_answer_now_requested() {
        Ok(build_answer_now_partial_result(
            query,
            requested_url,
            &context.web_state.allowed_sources,
        ))
    } else if let Some(q) = query {
        match await_with_request_control(
            crate::tools::web::search_query_with_progress(q, Some(6), |message| {
                emit_status(message);
            }),
            context.request_control,
        )
        .await
        {
            ControlledAwaitOutcome::Completed(Ok(result)) => Ok(result),
            ControlledAwaitOutcome::Completed(Err(search_error)) => {
                println!("[WebSearch] Primary query failed: {}", search_error);
                let mut final_error = search_error;
                let mut resolved: Option<crate::tools::web::WebSearchResult> = None;

                emit_status("Trying another reliable source".to_string());

                let local_candidates = crate::tools::web::local_safe_source_candidates(
                    q,
                    &context.web_state.attempted_domains,
                    3,
                );
                println!(
                    "[WebSearch] Local safe-source candidates: {}",
                    local_candidates.len()
                );

                for candidate in local_candidates {
                    if context.web_state.attempted_urls.contains(&candidate.url) {
                        continue;
                    }

                    mark_attempted_url(
                        &candidate.url,
                        &mut context.web_state.attempted_urls,
                        &mut context.web_state.attempted_domains,
                    );
                    emit_status("Trying another reliable source".to_string());

                    let mut allowed = HashMap::new();
                    allowed.insert(candidate.url.clone(), candidate.clone());

                    match await_with_request_control(
                        crate::tools::web::fetch_url_from_allowed_with_progress(
                            &candidate.url,
                            &allowed,
                            |message| {
                                emit_status(message);
                            },
                        ),
                        context.request_control,
                    )
                    .await
                    {
                        ControlledAwaitOutcome::Completed(Ok(result)) => {
                            let fallback_message = format!(
                                "Primary search failed; used trusted fallback source: {}.",
                                candidate.title
                            );
                            resolved =
                                Some(wrap_query_fallback_result(q, result, &fallback_message));
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
                                &context.web_state.allowed_sources,
                            ));
                            break;
                        }
                    }
                }

                if resolved.is_none() {
                    emit_status("Trying another reliable source".to_string());

                    let suggested_urls = match await_with_request_control(
                        suggest_fallback_urls(context.client, context.api_key, context.model, q, 6),
                        context.request_control,
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
                                &context.web_state.allowed_sources,
                            ));
                            Vec::new()
                        }
                    };

                    if resolved.is_none() {
                        let filtered_candidates =
                            crate::tools::web::filter_suggested_urls_to_safe_sources(
                                &suggested_urls,
                                &context.web_state.attempted_domains,
                                3,
                            );
                        println!(
                            "[WebSearch] Gemini suggested {} URLs, {} passed safe filtering",
                            suggested_urls.len(),
                            filtered_candidates.len()
                        );

                        for candidate in filtered_candidates {
                            if context.web_state.attempted_urls.contains(&candidate.url) {
                                continue;
                            }

                            mark_attempted_url(
                                &candidate.url,
                                &mut context.web_state.attempted_urls,
                                &mut context.web_state.attempted_domains,
                            );
                            emit_status("Trying another reliable source".to_string());

                            let mut allowed = HashMap::new();
                            allowed.insert(candidate.url.clone(), candidate.clone());

                            match await_with_request_control(
                                crate::tools::web::fetch_url_from_allowed_with_progress(
                                    &candidate.url,
                                    &allowed,
                                    |message| {
                                        emit_status(message);
                                    },
                                ),
                                context.request_control,
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
                                        &context.web_state.allowed_sources,
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
            ControlledAwaitOutcome::AnswerNow => Ok(build_answer_now_partial_result(
                Some(q),
                None,
                &context.web_state.allowed_sources,
            )),
        }
    } else if let Some(u) = requested_url {
        mark_attempted_url(
            u,
            &mut context.web_state.attempted_urls,
            &mut context.web_state.attempted_domains,
        );
        match await_with_request_control(
            crate::tools::web::fetch_url_from_allowed_with_progress(
                u,
                &context.web_state.allowed_sources,
                |message| {
                    emit_status(message);
                },
            ),
            context.request_control,
        )
        .await
        {
            ControlledAwaitOutcome::Completed(result) => result,
            ControlledAwaitOutcome::Cancelled => return Err("CANCELLED".to_string()),
            ControlledAwaitOutcome::AnswerNow => Ok(build_answer_now_partial_result(
                None,
                Some(u),
                &context.web_state.allowed_sources,
            )),
        }
    } else {
        Err("Tool call requires either `query` or `url`.".to_string())
    };

    let dispatch_result = match tool_result {
        Ok(result) => {
            merge_allowed_sources(&mut context.web_state.allowed_sources, &result);
            track_attempted_sources(
                &result.sources,
                &mut context.web_state.attempted_urls,
                &mut context.web_state.attempted_domains,
            );
            let done_message = result
                .message
                .clone()
                .unwrap_or_else(|| "Web search step completed.".to_string());
            ToolDispatchResult {
                response_value: serde_json::to_value(&result).unwrap_or_else(|_| {
                    json!({
                        "success": false,
                        "sources": [],
                        "message": "Serialization failure for tool output"
                    })
                }),
                follow_up_parts: Vec::new(),
                status: "done".to_string(),
                message: done_message,
                is_failure: false,
            }
        }
        Err(error_message) => ToolDispatchResult {
            response_value: json!({
                "mode": if requested_url.is_some() { "url" } else { "query" },
                "success": false,
                "sources": [],
                "context_markdown": "",
                "message": error_message
            }),
            follow_up_parts: Vec::new(),
            status: "error".to_string(),
            message: "Web search step failed.".to_string(),
            is_failure: true,
        },
    };

    Ok(dispatch_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::gemini::transport::types::GeminiFunctionCall;
    use serde_json::json;

    #[tokio::test]
    async fn unknown_tool_returns_error_result() {
        let client = reqwest::Client::new();
        let request_control = GeminiRequestControl::new();
        let mut web_state = WebToolDispatchState::default();
        let gemini_file_cache = std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let mut context = ToolDispatchContext {
            client: &client,
            api_key: "k",
            model: "m",
            chat_id: None,
            gemini_file_cache: &gemini_file_cache,
            request_control: &request_control,
            web_state: &mut web_state,
        };
        let call = GeminiFunctionCall {
            name: "nonexistent_tool".to_string(),
            args: json!({}),
        };

        let result = dispatch_tool_call(&call, &mut context, |_| {})
            .await
            .expect("dispatch should not fail");

        assert_eq!(result.status, "error");
        assert!(result.is_failure);
        assert_eq!(
            result
                .response_value
                .get("error_code")
                .and_then(|v| v.as_str()),
            Some("unknown_tool")
        );
    }

    #[tokio::test]
    async fn local_attachment_tool_requires_path() {
        let client = reqwest::Client::new();
        let request_control = GeminiRequestControl::new();
        let mut web_state = WebToolDispatchState::default();
        let gemini_file_cache = std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let mut context = ToolDispatchContext {
            client: &client,
            api_key: "k",
            model: "m",
            chat_id: None,
            gemini_file_cache: &gemini_file_cache,
            request_control: &request_control,
            web_state: &mut web_state,
        };
        let call = GeminiFunctionCall {
            name: "read_local_attachment_context".to_string(),
            args: json!({}),
        };

        let result = dispatch_tool_call(&call, &mut context, |_| {})
            .await
            .expect("dispatch should not fail");

        assert_eq!(result.status, "error");
        assert!(result.is_failure);
        assert_eq!(
            result
                .response_value
                .get("error_code")
                .and_then(|v| v.as_str()),
            Some("invalid_arguments")
        );
    }

    #[tokio::test]
    async fn web_search_dispatches_through_web_handler() {
        let client = reqwest::Client::new();
        let request_control = GeminiRequestControl::new();
        let mut web_state = WebToolDispatchState::default();
        let gemini_file_cache = std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let mut context = ToolDispatchContext {
            client: &client,
            api_key: "k",
            model: "m",
            chat_id: None,
            gemini_file_cache: &gemini_file_cache,
            request_control: &request_control,
            web_state: &mut web_state,
        };
        let call = GeminiFunctionCall {
            name: "web_search".to_string(),
            args: json!({}),
        };

        let result = dispatch_tool_call(&call, &mut context, |_| {})
            .await
            .expect("dispatch should not fail");

        assert_eq!(result.status, "error");
        assert!(result.is_failure);
        assert_eq!(
            result
                .response_value
                .get("message")
                .and_then(|v| v.as_str()),
            Some("Tool call requires either `query` or `url`.")
        );
    }
}
