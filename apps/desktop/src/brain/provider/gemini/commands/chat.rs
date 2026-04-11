// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use tauri::AppHandle;

use crate::brain::provider::gemini::agent::request_control::{
    register_request, remove_request, GeminiRequestControl,
};
use crate::brain::provider::gemini::agent::tool_dispatch::{
    dispatch_tool_call, ToolDispatchContext, WebToolDispatchState,
};
use crate::brain::provider::gemini::agent::tool_orchestrator::{
    build_system_instruction_with_tool_policy, tool_status_text, tool_step_id,
};
use crate::brain::provider::gemini::attachments::{
    build_attachment_preview_context, build_chat_attachment_catalog, build_interleaved_parts,
    extract_attachment_mentions, prepare_turn_attachments,
};
use crate::brain::provider::gemini::transport::streaming::{emit_event, stream_request_iteration};
use crate::brain::provider::gemini::transport::types::{
    GeminiContent, GeminiEvent, GeminiFileData, GeminiFunctionResponse, GeminiPart, GeminiRequest,
};

fn normalize_attachment_lookup_key(path: &str) -> String {
    let trimmed = path.trim();
    trimmed
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .map(str::trim)
        .unwrap_or(trimmed)
        .to_string()
}

fn attachment_lookup_aliases(path: &str) -> Vec<String> {
    let mut aliases = Vec::<String>::new();
    let mut push_unique = |value: String| {
        if !value.is_empty() && !aliases.iter().any(|existing| existing == &value) {
            aliases.push(value);
        }
    };

    let normalized = normalize_attachment_lookup_key(path);
    push_unique(normalized.clone());

    let normalized_without_current_dir = normalized
        .strip_prefix("./")
        .map(str::to_string)
        .unwrap_or_else(|| normalized.clone());
    push_unique(normalized_without_current_dir.clone());

    let normalized_path = Path::new(&normalized);
    if let Some(file_name) = normalized_path.file_name().and_then(|value| value.to_str()) {
        push_unique(file_name.to_string());
    }
    if let Some(stem) = normalized_path.file_stem().and_then(|value| value.to_str()) {
        push_unique(stem.to_string());
    }

    if let Ok(canonical) =
        crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal(
            &normalized,
        )
    {
        let canonical_str = canonical.to_string_lossy().to_string();
        push_unique(canonical_str.clone());

        let canonical_path = Path::new(&canonical_str);
        if let Some(file_name) = canonical_path.file_name().and_then(|value| value.to_str()) {
            push_unique(file_name.to_string());
        }
        if let Some(stem) = canonical_path.file_stem().and_then(|value| value.to_str()) {
            push_unique(stem.to_string());
        }
    }

    aliases
}

fn insert_attachment_display_name(
    map: &mut HashMap<String, String>,
    path: &str,
    display_name: &str,
) {
    for alias in attachment_lookup_aliases(path) {
        map.entry(alias).or_insert_with(|| display_name.to_string());
    }
}

fn find_attachment_display_name<'a>(
    path: &str,
    map: &'a HashMap<String, String>,
) -> Option<&'a str> {
    for alias in attachment_lookup_aliases(path) {
        if let Some(value) = map.get(&alias) {
            return Some(value.as_str());
        }
    }
    None
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
    chat_id: Option<String>,
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
        let mut attachment_display_name_by_path = HashMap::<String, String>::new();
        let tool_declarations = if allow_tools {
            Some(crate::brain::context::loader::load_gemini_tool_declarations()?)
        } else {
            None
        };
        let mut web_tool_state = WebToolDispatchState::default();

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
            let mut context_prompt = crate::brain::context::builder::build_turn_context(
                &img_desc, &first_msg, &history, &summary,
            );

            let mut composed_user_message = user_message.clone();
            let attachment_mentions = extract_attachment_mentions(&user_message);
            for mention in &attachment_mentions {
                if let Some(display_name) = mention
                    .display_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    insert_attachment_display_name(
                        &mut attachment_display_name_by_path,
                        &mention.path,
                        display_name,
                    );
                }
            }
            let prepared_attachments = prepare_turn_attachments(
                chat_id.as_deref(),
                &attachment_mentions,
                &api_key,
                &state.gemini_file_cache,
            )
            .await?;

            if let Some(preview_block) =
                build_attachment_preview_context(&prepared_attachments.preview_attachment_paths).await?
            {
                if !composed_user_message.trim().is_empty() {
                    composed_user_message.push_str("\n\n");
                }
                composed_user_message.push_str(&preview_block);
            }

            if let Some(attachment_catalog) = build_chat_attachment_catalog(chat_id.as_deref())? {
                context_prompt.push_str("\n\n");
                context_prompt.push_str(&attachment_catalog);
            }

            let mut parts = vec![
                GeminiPart {
                    text: Some(context_prompt),
                    ..Default::default()
                },
                GeminiPart {
                    text: Some(composed_user_message),
                    ..Default::default()
                },
            ];
            parts.extend(prepared_attachments.uploaded_parts);

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
                Some(
                    tool_declarations
                        .as_ref()
                        .ok_or_else(|| "Tool declarations not loaded".to_string())?
                        .clone(),
                )
            } else {
                None
            };

            let sys_instruction = build_system_instruction_with_tool_policy(
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

            let attachment_display_name = function_call
                .args
                .get("path")
                .and_then(|value| value.as_str())
                .and_then(|raw_path| {
                    find_attachment_display_name(raw_path, &attachment_display_name_by_path)
                });
            let status_text = tool_status_text(&function_call, attachment_display_name);
            let call_id = tool_step_id(iter, &function_call.name);
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
                    name: function_call.name.clone(),
                    args: function_call.args.clone(),
                    message: status_text.unwrap_or_default(),
                },
            );

            let mut dispatch_context = ToolDispatchContext {
                client: &client,
                api_key: &api_key,
                model: &model,
                chat_id: chat_id.as_deref(),
                gemini_file_cache: &state.gemini_file_cache,
                request_control: &request_control,
                web_state: &mut web_tool_state,
            };
            let dispatch_result = dispatch_tool_call(&function_call, &mut dispatch_context, |message| {
                emit_event(&app, &channel_id, GeminiEvent::ToolStatus { message });
            })
            .await?;

            emit_event(
                &app,
                &channel_id,
                GeminiEvent::ToolEnd {
                    id: call_id,
                    name: function_call.name.clone(),
                    status: dispatch_result.status.clone(),
                    result: dispatch_result.response_value.clone(),
                    message: dispatch_result.message.clone(),
                },
            );

            if dispatch_result.is_failure {
                consecutive_tool_failures += 1;
            } else {
                consecutive_tool_failures = 0;
            }

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
                            "Tools are unavailable right now, continuing with available context"
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
                        response: dispatch_result.response_value,
                    }),
                    ..Default::default()
                }],
            });
            if !dispatch_result.follow_up_parts.is_empty() {
                contents.push(GeminiContent {
                    role: "user".to_string(),
                    parts: dispatch_result.follow_up_parts,
                });
            }
        }

        Err("Maximum tool iterations reached without final response.".to_string())
    }
    .await;

    remove_request(&channel_id).await;

    result
}
