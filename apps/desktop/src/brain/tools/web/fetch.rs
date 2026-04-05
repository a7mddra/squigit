// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use reqwest::{header, StatusCode};
use std::collections::HashMap;
use url::Url;

use super::constants::{MAX_FETCH_BYTES, MAX_REDIRECTS, MAX_SUMMARY_WORDS};
use super::favicon::{cache_favicons_for_sources, citation_source};
use super::html::{clean_page_text, compact_summary, extract_title};
use super::retry::{emit_progress, with_retries_with_progress};
use super::transport::{read_capped_response_body, send_with_transport, TransportClients};
use super::types::{CitationSource, SearchError, SearchFailureClass, WebSearchResult};
use super::url_utils::{canonicalize_url, ensure_public_target};

pub(crate) async fn fetch_html_with_redirects(
    url: &str,
    clients: &TransportClients,
) -> Result<String, SearchError> {
    let mut current = Url::parse(url).map_err(|_| {
        SearchError::fatal(
            SearchFailureClass::InvalidUrl,
            format!("Invalid URL: {}", url),
        )
    })?;

    for hop in 0..=MAX_REDIRECTS {
        ensure_public_target(&current).await?;
        let current_url = current.clone();

        let (response, route) = send_with_transport(clients, move |client| {
            client
                .get(current_url.clone())
                .header(header::ACCEPT, "text/html,application/xhtml+xml")
        })
        .await?;

        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| {
                    SearchError::fatal(
                        SearchFailureClass::HttpStatus,
                        "Redirect missing Location header",
                    )
                })?;
            let next = current.join(location).map_err(|e| {
                SearchError::fatal(
                    SearchFailureClass::InvalidUrl,
                    format!("Bad redirect URL: {}", e),
                )
            })?;
            current = next;
            if hop == MAX_REDIRECTS {
                return Err(SearchError::fatal(
                    SearchFailureClass::HttpStatus,
                    "Too many redirects",
                ));
            }
            continue;
        }

        if !response.status().is_success() {
            let status = response.status();
            let retriable = status == StatusCode::TOO_MANY_REQUESTS
                || status == StatusCode::BAD_GATEWAY
                || status == StatusCode::SERVICE_UNAVAILABLE
                || status == StatusCode::GATEWAY_TIMEOUT
                || status.is_server_error();
            let message = format!(
                "HTTP {} while fetching URL via {}",
                status.as_u16(),
                route.as_str()
            );

            return if retriable {
                Err(SearchError::retriable(
                    SearchFailureClass::HttpStatus,
                    message,
                ))
            } else {
                Err(SearchError::fatal(SearchFailureClass::HttpStatus, message))
            };
        }

        return read_capped_response_body(response, MAX_FETCH_BYTES).await;
    }

    Err(SearchError::fatal(
        SearchFailureClass::HttpStatus,
        "Too many redirects",
    ))
}

pub(crate) async fn run_url_fetch_once(
    url: &str,
    clients: &TransportClients,
) -> Result<WebSearchResult, SearchError> {
    let canonical = canonicalize_url(url)?;
    let html = fetch_html_with_redirects(&canonical, clients).await?;
    let text = clean_page_text(&html);
    if text.trim().is_empty() {
        return Err(SearchError::retriable(
            SearchFailureClass::Parse,
            "Fetched page returned no readable text",
        ));
    }

    let title = extract_title(&html, &canonical);
    let summary = compact_summary(&text, MAX_SUMMARY_WORDS);
    let mut sources = vec![citation_source(title, canonical.clone(), summary.clone())];
    cache_favicons_for_sources(&mut sources).await;

    let context = format!(
        "[Fetched page: {}]\n- {}\n\n{}",
        canonical,
        if summary.is_empty() {
            "(No short summary available)"
        } else {
            &summary
        },
        text
    );

    Ok(WebSearchResult {
        mode: "url".to_string(),
        query: None,
        requested_url: Some(canonical),
        context_markdown: context,
        sources,
        success: true,
        message: None,
    })
}

pub(crate) fn build_degraded_url_result(
    canonical_url: &str,
    source: &CitationSource,
    fetch_error: &SearchError,
) -> WebSearchResult {
    let summary = if source.summary.trim().is_empty() {
        "(No prior snippet available)".to_string()
    } else {
        source.summary.trim().to_string()
    };

    let context = format!(
        "[Fetched page unavailable: {}]\n- {}\n\nUsing snippet from prior search result:\n{}",
        canonical_url, source.title, summary
    );

    WebSearchResult {
        mode: "url".to_string(),
        query: None,
        requested_url: Some(canonical_url.to_string()),
        context_markdown: context,
        sources: vec![source.clone()],
        success: true,
        message: Some(format!(
            "Full page fetch failed ({}); returned snippet-based fallback.",
            fetch_error.kind.as_str()
        )),
    }
}

pub fn collect_allowed_sources(result: &WebSearchResult) -> HashMap<String, CitationSource> {
    let mut out = HashMap::<String, CitationSource>::new();
    for source in &result.sources {
        if let Ok(canonical) = canonicalize_url(&source.url) {
            out.insert(canonical, source.clone());
        }
    }
    out
}

pub async fn fetch_url_from_allowed(
    url: &str,
    allowed_sources: &HashMap<String, CitationSource>,
) -> Result<WebSearchResult, String> {
    fetch_url_from_allowed_with_progress(url, allowed_sources, |_| {}).await
}

pub async fn fetch_url_from_allowed_with_progress<F>(
    url: &str,
    allowed_sources: &HashMap<String, CitationSource>,
    mut progress: F,
) -> Result<WebSearchResult, String>
where
    F: FnMut(String) + Send,
{
    let canonical = canonicalize_url(url).map_err(|e| e.public_message())?;
    let source = allowed_sources.get(&canonical).ok_or_else(|| {
        "Blocked URL fetch: URL must come from a previous search result in this turn".to_string()
    })?;

    let clients = TransportClients::build().map_err(|e| e.public_message())?;
    let mut progress_ref: Option<&mut (dyn FnMut(String) + Send)> = Some(&mut progress);

    emit_progress(
        &mut progress_ref,
        format!("Fetching page content: {}", canonical),
    );

    match with_retries_with_progress(
        "Fetch",
        || run_url_fetch_once(&canonical, &clients),
        &mut progress_ref,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(fetch_error) => {
            println!(
                "[WebSearch] url_fetch_failed [{}]: {}",
                fetch_error.kind.as_str(),
                fetch_error.message
            );
            emit_progress(
                &mut progress_ref,
                "Couldn't open one source, trying another".to_string(),
            );
            Ok(build_degraded_url_result(&canonical, source, &fetch_error))
        }
    }
}
