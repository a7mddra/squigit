// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use super::request_control::GeminiRequestControl;
use super::types::GeminiFunctionCall;

pub(crate) fn tool_step_id(iter: usize) -> String {
    format!("web-search-call-{}", iter + 1)
}

pub(crate) fn tool_status_text(function_call: &GeminiFunctionCall) -> Option<String> {
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

pub(crate) fn build_system_instruction_with_search_policy(
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

pub(crate) fn merge_allowed_sources(
    allowed_sources: &mut HashMap<String, crate::brain::tools::duckduckgo::CitationSource>,
    result: &crate::brain::tools::duckduckgo::WebSearchResult,
) {
    for (url, source) in crate::brain::tools::duckduckgo::collect_allowed_sources(result) {
        allowed_sources.insert(url, source);
    }
}

pub(crate) fn track_attempted_sources(
    sources: &[crate::brain::tools::duckduckgo::CitationSource],
    attempted_urls: &mut HashSet<String>,
    attempted_domains: &mut HashSet<String>,
) {
    for source in sources {
        attempted_urls.insert(source.url.clone());
        if let Some(domain) = crate::brain::tools::duckduckgo::domain_from_url(&source.url) {
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
    if let Some(domain) = crate::brain::tools::duckduckgo::domain_from_url(raw_url) {
        attempted_domains.insert(domain);
    }
}

pub(crate) fn wrap_query_fallback_result(
    query: &str,
    mut result: crate::brain::tools::duckduckgo::WebSearchResult,
    fallback_message: &str,
) -> crate::brain::tools::duckduckgo::WebSearchResult {
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
    allowed_sources: &HashMap<String, crate::brain::tools::duckduckgo::CitationSource>,
    max_sources: usize,
) -> Vec<crate::brain::tools::duckduckgo::CitationSource> {
    let mut sources = allowed_sources.values().cloned().collect::<Vec<_>>();
    sources.sort_by(|a, b| a.url.cmp(&b.url));
    sources.truncate(max_sources);
    sources
}

fn build_answer_now_context_markdown(
    mode_label: &str,
    sources: &[crate::brain::tools::duckduckgo::CitationSource],
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
    allowed_sources: &HashMap<String, crate::brain::tools::duckduckgo::CitationSource>,
) -> crate::brain::tools::duckduckgo::WebSearchResult {
    let mode = if requested_url.is_some() {
        "url"
    } else {
        "query"
    };
    let sources = collect_answer_now_sources(allowed_sources, 6);
    crate::brain::tools::duckduckgo::WebSearchResult {
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
