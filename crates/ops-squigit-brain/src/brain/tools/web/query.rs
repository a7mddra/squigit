// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use reqwest::{header, StatusCode};

use super::constants::{DDG_SEARCH_URL, DEFAULT_MAX_RESULTS, MAX_FETCH_BYTES, MOJEEK_SEARCH_URL};
use super::favicon::cache_favicons_for_sources;
use super::html::{
    build_query_result, looks_like_ddg_challenge_page, looks_like_mojeek_block_page,
    parse_ddg_results, parse_mojeek_results, rerank_sources,
};
use super::retry::{emit_progress, with_retries_with_progress};
use super::transport::{read_capped_response_body, send_with_transport, TransportClients};
use super::types::{
    CitationSource, SearchBackend, SearchError, SearchFailureClass, WebSearchResult,
};
use super::url_utils::encode_query;

pub(crate) async fn run_backend_query_once(
    backend: SearchBackend,
    query: &str,
    max_results: usize,
    clients: &TransportClients,
) -> Result<Vec<CitationSource>, SearchError> {
    let encoded = encode_query(query);
    let search_url = match backend {
        SearchBackend::DuckDuckGo => format!("{}{}", DDG_SEARCH_URL, encoded),
        SearchBackend::Mojeek => format!("{}{}", MOJEEK_SEARCH_URL, encoded),
    };

    let (response, route) = send_with_transport(clients, move |client| {
        client
            .get(search_url.clone())
            .header(header::ACCEPT, "text/html")
    })
    .await?;

    if !response.status().is_success() {
        let status = response.status();
        let status_message = format!(
            "{} returned HTTP {} via {}",
            backend.as_str(),
            status.as_u16(),
            route.as_str()
        );

        if backend == SearchBackend::Mojeek && status == StatusCode::FORBIDDEN {
            return Err(SearchError::retriable(
                SearchFailureClass::Challenge,
                "Mojeek blocked automated requests (403)",
            ));
        }

        return if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            Err(SearchError::retriable(
                SearchFailureClass::HttpStatus,
                status_message,
            ))
        } else {
            Err(SearchError::fatal(
                SearchFailureClass::HttpStatus,
                status_message,
            ))
        };
    }

    let html = read_capped_response_body(response, MAX_FETCH_BYTES).await?;

    if backend == SearchBackend::DuckDuckGo && looks_like_ddg_challenge_page(&html) {
        return Err(SearchError::retriable(
            SearchFailureClass::Challenge,
            "DuckDuckGo temporarily blocked automated requests (challenge page)",
        ));
    }
    if backend == SearchBackend::Mojeek && looks_like_mojeek_block_page(&html) {
        return Err(SearchError::retriable(
            SearchFailureClass::Challenge,
            "Mojeek temporarily blocked automated requests",
        ));
    }

    let parsed = match backend {
        SearchBackend::DuckDuckGo => parse_ddg_results(&html, max_results),
        SearchBackend::Mojeek => parse_mojeek_results(&html, max_results),
    };

    if parsed.is_empty() {
        return Err(SearchError::retriable(
            SearchFailureClass::NoResults,
            format!("{} returned no usable results", backend.as_str()),
        ));
    }

    Ok(rerank_sources(query, parsed, max_results))
}

pub async fn search_query(
    query: &str,
    max_results: Option<usize>,
) -> Result<WebSearchResult, String> {
    search_query_with_progress(query, max_results, |_| {}).await
}

pub async fn search_query_with_progress<F>(
    query: &str,
    max_results: Option<usize>,
    mut progress: F,
) -> Result<WebSearchResult, String>
where
    F: FnMut(String) + Send,
{
    let q = query.trim();
    if q.is_empty() {
        return Err("No query provided".to_string());
    }

    let limit = max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .clamp(1, DEFAULT_MAX_RESULTS);

    let clients = TransportClients::build().map_err(|e| e.public_message())?;
    let mut last_error: Option<SearchError> = None;
    let mut progress_ref: Option<&mut (dyn FnMut(String) + Send)> = Some(&mut progress);

    for backend in [SearchBackend::Mojeek, SearchBackend::DuckDuckGo] {
        let backend_name = match backend {
            SearchBackend::DuckDuckGo => "DuckDuckGo",
            SearchBackend::Mojeek => "Mojeek",
        };
        emit_progress(
            &mut progress_ref,
            "Searching for relevant sources".to_string(),
        );

        match with_retries_with_progress(
            backend_name,
            || run_backend_query_once(backend, q, limit, &clients),
            &mut progress_ref,
        )
        .await
        {
            Ok(mut sources) => {
                cache_favicons_for_sources(&mut sources).await;
                println!(
                    "[WebSearch] backend={} success results={}",
                    backend.as_str(),
                    sources.len()
                );
                emit_progress(
                    &mut progress_ref,
                    format!("{}: found {} results", backend_name, sources.len()),
                );
                return Ok(build_query_result(q, sources, None));
            }
            Err(error) => {
                println!(
                    "[WebSearch] backend={} failed [{}]: {}",
                    backend.as_str(),
                    error.kind.as_str(),
                    error.message
                );
                emit_progress(&mut progress_ref, "Trying another source".to_string());
                last_error = Some(error);
            }
        }
    }

    emit_progress(
        &mut progress_ref,
        "Search is unavailable right now.".to_string(),
    );
    Err(last_error
        .map(|e| e.public_message())
        .unwrap_or_else(|| "[other] Search unavailable".to_string()))
}
