// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde_json::json;
use std::collections::{HashMap, HashSet};
use tauri::AppHandle;

use crate::brain::provider::gemini::agent::request_control::{
    register_request, remove_request, GeminiRequestControl,
};
use crate::brain::provider::gemini::agent::tool_orchestrator::{
    await_with_request_control, build_answer_now_partial_result,
    build_system_instruction_with_search_policy, mark_attempted_url, merge_allowed_sources,
    tool_status_text, tool_step_id, track_attempted_sources, wrap_query_fallback_result,
    ControlledAwaitOutcome,
};
use crate::brain::provider::gemini::attachments::build_interleaved_parts;
use crate::brain::provider::gemini::transport::streaming::{emit_event, stream_request_iteration};
use crate::brain::provider::gemini::transport::types::{
    GeminiContent, GeminiEvent, GeminiFileData, GeminiFunctionResponse, GeminiPart, GeminiRequest,
};
use crate::brain::tools::web::suggest_fallback_urls;

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
    const MAX_OUTPUT_TOKENS: usize = 2048;

    let result = async {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );

        let request_control = GeminiRequestControl::new();
        register_request(channel_id.clone(), request_control.clone()).await;

        let mut allow_tools = !is_initial_turn;
        let mut tool_calls = 0usize;
        let mut consecutive_tool_failures = 0usize;
        let web_search_tool_declaration = if allow_tools {
            Some(crate::brain::context::loader::load_web_search_tool_declaration()?)
        } else {
            None
        };
        let mut allowed_sources = HashMap::<String, crate::brain::tools::web::CitationSource>::new();
        let mut attempted_urls = HashSet::<String>::new();
        let mut attempted_domains = HashSet::<String>::new();

        // Build conversation contents once; then append tool call/response turns as needed.
        let mut contents: Vec<GeminiContent> = if is_initial_turn {
            let system_prompt = crate::brain::context::builder::build_initial_system_prompt()?;
            let mut parts = vec![];

            if let Some(path) = image_path.clone() {
                let file_ref =
                    crate::brain::provider::gemini::attachments::ensure_file_uploaded(&api_key, &path, &state.gemini_file_cache)
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
                crate::brain::context::builder::build_turn_context(&img_desc, &first_msg, &history, &summary);

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
                generation_config: Some(json!({
                    "maxOutputTokens": MAX_OUTPUT_TOKENS
                })),
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
                    crate::brain::tools::web::search_query_with_progress(q, Some(6), |message| {
                        emit_event(&app, &channel_id, GeminiEvent::ToolStatus { message });
                    }),
                    &request_control,
                )
                .await
                {
                    ControlledAwaitOutcome::Completed(Ok(result)) => Ok(result),
                    ControlledAwaitOutcome::Completed(Err(search_error)) => {
                        println!("[WebSearch] Primary query failed: {}", search_error);
                        let mut final_error = search_error;
                        let mut resolved: Option<crate::brain::tools::web::WebSearchResult> = None;

                        emit_event(
                            &app,
                            &channel_id,
                            GeminiEvent::ToolStatus {
                                message: "Trying another reliable source".to_string(),
                            },
                        );

                        let local_candidates = crate::brain::tools::web::local_safe_source_candidates(
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
                                crate::brain::tools::web::fetch_url_from_allowed_with_progress(
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
                                let filtered_candidates = crate::brain::tools::web::filter_suggested_urls_to_safe_sources(
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
                                        crate::brain::tools::web::fetch_url_from_allowed_with_progress(
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
                    crate::brain::tools::web::fetch_url_from_allowed_with_progress(
                        u,
                        &allowed_sources,
                        |message| {
                            emit_event(&app, &channel_id, GeminiEvent::ToolStatus { message });
                        },
                    ),
                    &request_control,
                )
                .await
                {
                    ControlledAwaitOutcome::Completed(result) => result,
                    ControlledAwaitOutcome::Cancelled => return Err("CANCELLED".to_string()),
                    ControlledAwaitOutcome::AnswerNow => {
                        Ok(build_answer_now_partial_result(None, Some(u), &allowed_sources))
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

    remove_request(&channel_id).await;

    result
}
