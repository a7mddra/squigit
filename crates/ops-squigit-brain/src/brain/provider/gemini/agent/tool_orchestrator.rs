// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use super::request_control::GeminiRequestControl;
use crate::brain::provider::gemini::transport::types::GeminiFunctionCall;

fn displayable_attachment_name(raw_value: &str) -> Option<String> {
    let file_name = std::path::Path::new(raw_value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(raw_value)
        .trim();
    if file_name.is_empty() {
        return None;
    }

    let stem = std::path::Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(file_name)
        .trim();
    if stem.len() >= 16 && stem.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    Some(file_name.to_string())
}

pub(crate) fn tool_step_id(iter: usize, tool_name: &str) -> String {
    let normalized_name = tool_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    format!("{}-call-{}", normalized_name, iter + 1)
}

pub(crate) fn tool_status_text(
    function_call: &GeminiFunctionCall,
    attachment_display_name: Option<&str>,
) -> Option<String> {
    if function_call.name == "read_local_attachment_context" {
        if let Some(display_name) = attachment_display_name
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return Some(format!("Reading local context from {}", display_name));
        }

        let path = function_call
            .args
            .get("path")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if let Some(path_value) = path {
            if let Some(file_name) = displayable_attachment_name(path_value) {
                return Some(format!("Reading local context from {}", file_name));
            }
        }
        return Some("Reading local attachment context".to_string());
    }

    if function_call.name == "recall_chat_attachment" {
        if let Some(display_name) = attachment_display_name
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            return Some(format!("Recalling {}", display_name));
        }

        let target = function_call
            .args
            .get("target")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if let Some(target_value) = target {
            if let Some(file_name) = displayable_attachment_name(target_value) {
                return Some(format!("Recalling {}", file_name));
            }
        }
        return Some("Recalling a previous attachment".to_string());
    }

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

pub(crate) fn build_system_instruction_with_tool_policy(
    user_name: &str,
    user_email: &str,
    image_brief: &str,
    tools_enabled: bool,
) -> Result<String, String> {
    let mut instruction = crate::brain::context::builder::build_system_instruction(
        user_name,
        user_email,
        image_brief,
    )?;

    if tools_enabled {
        instruction.push_str(
            "\n\n## Tool Usage Policy\n\
             - If the user asks for current, time-sensitive, or uncertain facts, call `web_search`.\n\
             - If the user asks about attached local text or code content, call `read_local_attachment_context`.\n\
             - If the user asks a complex, exact, or follow-up question about a prior local code/text attachment, call `read_local_attachment_context` again using the exact path from attachment references or the chat attachment catalog.\n\
             - Secondary uploaded files from this chat may only be used when the user attaches them in this turn or when you explicitly call `recall_chat_attachment`.\n\
             - Use the chat attachment catalog in context to pick the right tool: `read_local_attachment_context` for `text_local`, `recall_chat_attachment` for `image_upload` or `document_upload`.\n\
             - If the user asks for page-specific, OCR, quote-exact, transcription, chart-reading, or slide/sheet/section-specific details from a prior uploaded image or document, you must call `recall_chat_attachment` before answering.\n\
             - Never answer exact file-grounded questions from `image_brief`, rolling summaries, or path references alone when a prior uploaded image/document is needed.\n\
             - Never answer exact code/text questions from summaries alone when a local attachment can be re-read with `read_local_attachment_context`.\n\
             - For PDF, Word, spreadsheet, slide, and similar document attachments, rely on the attached Gemini file directly (do not call `read_local_attachment_context` for documents).\n\
             - If greeting/chit-chat, do not call tools.\n\
             - Never invent URLs or sources.\n\
             - When using `url`, only fetch URLs from prior search results in this turn.\n\
             - If one search pass is too shallow, call `web_search` again with a refined query.\n\
             - For local attachments, use paths exactly as provided by the user/tool context.\n\
             - If web search fails repeatedly, answer from model knowledge and clearly state web search was unavailable.",
        );
    }

    Ok(instruction)
}

pub(crate) fn merge_allowed_sources(
    allowed_sources: &mut HashMap<String, crate::brain::tools::web::CitationSource>,
    result: &crate::brain::tools::web::WebSearchResult,
) {
    for (url, source) in crate::brain::tools::web::collect_allowed_sources(result) {
        allowed_sources.insert(url, source);
    }
}

pub(crate) fn track_attempted_sources(
    sources: &[crate::brain::tools::web::CitationSource],
    attempted_urls: &mut HashSet<String>,
    attempted_domains: &mut HashSet<String>,
) {
    for source in sources {
        attempted_urls.insert(source.url.clone());
        if let Some(domain) = crate::brain::tools::web::domain_from_url(&source.url) {
            attempted_domains.insert(domain);
        }
    }
}

pub(crate) fn mark_attempted_url(
    raw_url: &str,
    attempted_urls: &mut HashSet<String>,
    attempted_domains: &mut HashSet<String>,
) {
    attempted_urls.insert(raw_url.to_string());
    if let Some(domain) = crate::brain::tools::web::domain_from_url(raw_url) {
        attempted_domains.insert(domain);
    }
}

pub(crate) fn wrap_query_fallback_result(
    query: &str,
    mut result: crate::brain::tools::web::WebSearchResult,
    fallback_message: &str,
) -> crate::brain::tools::web::WebSearchResult {
    result.mode = "query".to_string();
    result.query = Some(query.trim().to_string());
    result.requested_url = None;
    if result.message.is_none() {
        result.message = Some(fallback_message.to_string());
    }
    result
}

pub(crate) enum ControlledAwaitOutcome<T> {
    Completed(T),
    Cancelled,
    AnswerNow,
}

pub(crate) async fn await_with_request_control<T>(
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
    allowed_sources: &HashMap<String, crate::brain::tools::web::CitationSource>,
    max_sources: usize,
) -> Vec<crate::brain::tools::web::CitationSource> {
    let mut sources = allowed_sources.values().cloned().collect::<Vec<_>>();
    sources.sort_by(|a, b| a.url.cmp(&b.url));
    sources.truncate(max_sources);
    sources
}

fn build_answer_now_context_markdown(
    mode_label: &str,
    sources: &[crate::brain::tools::web::CitationSource],
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

pub(crate) fn build_answer_now_partial_result(
    query: Option<&str>,
    requested_url: Option<&str>,
    allowed_sources: &HashMap<String, crate::brain::tools::web::CitationSource>,
) -> crate::brain::tools::web::WebSearchResult {
    let mode = if requested_url.is_some() {
        "url"
    } else {
        "query"
    };
    let sources = collect_answer_now_sources(allowed_sources, 6);
    crate::brain::tools::web::WebSearchResult {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_step_id_uses_tool_name() {
        assert_eq!(tool_step_id(0, "web_search"), "web-search-call-1");
        assert_eq!(
            tool_step_id(1, "read_local_attachment_context"),
            "read-local-attachment-context-call-2"
        );
    }

    #[test]
    fn tool_status_text_for_web_search_query() {
        let call = GeminiFunctionCall {
            name: "web_search".to_string(),
            args: json!({ "query": "rust async" }),
        };
        assert_eq!(
            tool_status_text(&call, None),
            Some("Searching for relevant sources".to_string())
        );
    }

    #[test]
    fn tool_status_text_for_local_attachment() {
        let call = GeminiFunctionCall {
            name: "read_local_attachment_context".to_string(),
            args: json!({ "path": "/tmp/report.pdf" }),
        };
        assert_eq!(
            tool_status_text(&call, None),
            Some("Reading local context from report.pdf".to_string())
        );
    }

    #[test]
    fn tool_status_text_prefers_display_name_when_available() {
        let call = GeminiFunctionCall {
            name: "read_local_attachment_context".to_string(),
            args: json!({ "path": "objects/ab/abcdef123.pdf" }),
        };
        assert_eq!(
            tool_status_text(&call, Some("Quarterly Report.pdf")),
            Some("Reading local context from Quarterly Report.pdf".to_string())
        );
    }

    #[test]
    fn tool_status_text_prefers_display_name_for_recall() {
        let call = GeminiFunctionCall {
            name: "recall_chat_attachment".to_string(),
            args: json!({ "target": "objects/ab/abcdef123.pdf" }),
        };
        assert_eq!(
            tool_status_text(&call, Some("Quarterly Report.pdf")),
            Some("Recalling Quarterly Report.pdf".to_string())
        );
    }

    #[test]
    fn tool_status_text_hides_hashy_attachment_names() {
        let call = GeminiFunctionCall {
            name: "read_local_attachment_context".to_string(),
            args: json!({ "path": "objects/ab/00f8e1ec68a90d7d33ed18d6e44a4f36.rs" }),
        };
        assert_eq!(
            tool_status_text(&call, None),
            Some("Reading local attachment context".to_string())
        );
    }

    #[test]
    fn tool_policy_mentions_rereading_prior_local_attachments() {
        let instruction =
            build_system_instruction_with_tool_policy("", "", "", true).expect("policy");
        assert!(instruction.contains("prior local code/text attachment"));
        assert!(instruction.contains("`read_local_attachment_context` again"));
        assert!(instruction.contains("`text_local`"));
        assert!(instruction.contains("exact code/text questions"));
    }
}
