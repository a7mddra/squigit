// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::StreamExt;
use reqwest::redirect::Policy;
use std::time::Duration;

use super::constants::{CONNECT_TIMEOUT_SECS, REQUEST_TIMEOUT_SECS};
use super::types::{SearchError, SearchFailureClass, TransportRoute};

#[derive(Debug, Clone)]
pub(crate) struct TransportClients {
    direct: reqwest::Client,
    proxy: Option<reqwest::Client>,
}

impl TransportClients {
    pub(crate) fn build() -> Result<Self, SearchError> {
        let direct = build_client(true)?;
        let proxy = if has_proxy_env() {
            Some(build_client(false)?)
        } else {
            None
        };

        Ok(Self { direct, proxy })
    }

    pub(crate) fn route_order(&self) -> [TransportRoute; 2] {
        if self.proxy.is_some() {
            [TransportRoute::Proxy, TransportRoute::Direct]
        } else {
            [TransportRoute::Direct, TransportRoute::Direct]
        }
    }

    pub(crate) fn client_for(&self, route: TransportRoute) -> Option<&reqwest::Client> {
        match route {
            TransportRoute::Direct => Some(&self.direct),
            TransportRoute::Proxy => self.proxy.as_ref(),
        }
    }
}

pub(crate) fn user_agent() -> &'static str {
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36"
}

fn has_proxy_env() -> bool {
    [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ]
    .iter()
    .any(|key| {
        std::env::var(key)
            .ok()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    })
}

fn build_client(no_proxy: bool) -> Result<reqwest::Client, SearchError> {
    let mut builder = reqwest::Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .user_agent(user_agent());

    if no_proxy {
        builder = builder.no_proxy();
    }

    builder.build().map_err(|e| {
        SearchError::fatal(
            SearchFailureClass::Other,
            format!("HTTP client init failed: {}", e),
        )
    })
}

fn classify_reqwest_error(error: &reqwest::Error, route: TransportRoute) -> SearchError {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();

    if error.is_timeout() {
        if lower.contains("connect") || lower.contains("dns") {
            return SearchError::retriable(
                SearchFailureClass::ConnectTimeout,
                format!(
                    "Request timed out during connect ({}): {}",
                    route.as_str(),
                    message
                ),
            );
        }
        return SearchError::retriable(
            SearchFailureClass::ReadTimeout,
            format!(
                "Request timed out during read ({}): {}",
                route.as_str(),
                message
            ),
        );
    }

    if error.is_connect() {
        if route == TransportRoute::Proxy || lower.contains("proxy") || lower.contains("socks") {
            return SearchError::retriable(
                SearchFailureClass::ProxyTransport,
                format!("Proxy transport failure: {}", message),
            );
        }
        if lower.contains("dns") || lower.contains("lookup") {
            return SearchError::retriable(
                SearchFailureClass::Dns,
                format!("DNS/connect failure: {}", message),
            );
        }
        return SearchError::retriable(
            SearchFailureClass::ConnectTimeout,
            format!("Connection failure: {}", message),
        );
    }

    if lower.contains("dns") || lower.contains("lookup") {
        return SearchError::retriable(
            SearchFailureClass::Dns,
            format!("DNS failure: {}", message),
        );
    }

    SearchError::retriable(
        SearchFailureClass::Other,
        format!("Transport failure: {}", message),
    )
}

fn can_fallback_to_direct(kind: SearchFailureClass) -> bool {
    matches!(
        kind,
        SearchFailureClass::ProxyTransport
            | SearchFailureClass::ConnectTimeout
            | SearchFailureClass::ReadTimeout
            | SearchFailureClass::Dns
            | SearchFailureClass::Other
    )
}

pub(crate) async fn send_with_transport<F>(
    clients: &TransportClients,
    mut request_factory: F,
) -> Result<(reqwest::Response, TransportRoute), SearchError>
where
    F: FnMut(&reqwest::Client) -> reqwest::RequestBuilder,
{
    let routes = clients.route_order();
    let mut last_error: Option<SearchError> = None;

    for (idx, route) in routes.iter().copied().enumerate() {
        let Some(client) = clients.client_for(route) else {
            continue;
        };

        match request_factory(client).send().await {
            Ok(response) => return Ok((response, route)),
            Err(error) => {
                let mapped = classify_reqwest_error(&error, route);
                let has_next = idx + 1 < routes.len();
                if route == TransportRoute::Proxy && has_next && can_fallback_to_direct(mapped.kind)
                {
                    println!(
                        "[WebSearch] transport={} failed [{}]: {}; retrying direct",
                        route.as_str(),
                        mapped.kind.as_str(),
                        mapped.message
                    );
                    last_error = Some(mapped);
                    continue;
                }
                return Err(mapped);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        SearchError::retriable(
            SearchFailureClass::Other,
            "No transport route available for request",
        )
    }))
}

pub(crate) async fn read_capped_response_body(
    response: reqwest::Response,
    max_bytes: usize,
) -> Result<String, SearchError> {
    let mut stream = response.bytes_stream();
    let mut total = 0usize;
    let mut bytes = Vec::<u8>::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            let lower = e.to_string().to_ascii_lowercase();
            if lower.contains("timed out") {
                SearchError::retriable(
                    SearchFailureClass::ReadTimeout,
                    format!("Read body timed out: {}", e),
                )
            } else {
                SearchError::retriable(
                    SearchFailureClass::Other,
                    format!("Read body failed: {}", e),
                )
            }
        })?;

        total += chunk.len();
        if total > max_bytes {
            let allowed = chunk.len().saturating_sub(total - max_bytes);
            bytes.extend_from_slice(&chunk[..allowed]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(String::from_utf8_lossy(&bytes).to_string())
}
