// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::{future::join_all, StreamExt};
use ops_chat_storage::ChatStorage;
use ops_profile_store::ProfileStore;
use rand::Rng;
use regex::Regex;
use reqwest::{header, redirect::Policy, StatusCode};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv6Addr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

const DDG_SEARCH_URL: &str = "https://html.duckduckgo.com/html/?q=";
const MOJEEK_SEARCH_URL: &str = "https://www.mojeek.com/search?q=";
const DEFAULT_MAX_RESULTS: usize = 6;
const MAX_REDIRECTS: usize = 5;
const MAX_FETCH_BYTES: usize = 320 * 1024;
const MAX_FETCH_CHARS: usize = 12_000;
const MAX_SUMMARY_WORDS: usize = 50;
const MAX_RETRIES: usize = 2;
const REQUEST_TIMEOUT_SECS: u64 = 15;
const CONNECT_TIMEOUT_SECS: u64 = 8;
const FAVICON_TIMEOUT_SECS: u64 = 2;
const FAVICON_CONNECT_TIMEOUT_SECS: u64 = 2;
const MAX_FAVICON_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CitationSource {
    pub title: String,
    pub url: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebSearchResult {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_url: Option<String>,
    pub context_markdown: String,
    pub sources: Vec<CitationSource>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchFailureClass {
    ProxyTransport,
    ConnectTimeout,
    ReadTimeout,
    Challenge,
    HttpStatus,
    NoResults,
    Dns,
    InvalidUrl,
    BlockedTarget,
    Parse,
    Other,
}

impl SearchFailureClass {
    fn as_str(self) -> &'static str {
        match self {
            SearchFailureClass::ProxyTransport => "proxy_transport",
            SearchFailureClass::ConnectTimeout => "connect_timeout",
            SearchFailureClass::ReadTimeout => "read_timeout",
            SearchFailureClass::Challenge => "challenge",
            SearchFailureClass::HttpStatus => "http_status",
            SearchFailureClass::NoResults => "no_results",
            SearchFailureClass::Dns => "dns",
            SearchFailureClass::InvalidUrl => "invalid_url",
            SearchFailureClass::BlockedTarget => "blocked_target",
            SearchFailureClass::Parse => "parse",
            SearchFailureClass::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
struct SearchError {
    kind: SearchFailureClass,
    message: String,
    retriable: bool,
}

impl SearchError {
    fn fatal(kind: SearchFailureClass, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retriable: false,
        }
    }

    fn retriable(kind: SearchFailureClass, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retriable: true,
        }
    }

    fn public_message(&self) -> String {
        format!("[{}] {}", self.kind.as_str(), self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportRoute {
    Direct,
    Proxy,
}

impl TransportRoute {
    fn as_str(self) -> &'static str {
        match self {
            TransportRoute::Direct => "direct",
            TransportRoute::Proxy => "proxy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchBackend {
    DuckDuckGo,
    Mojeek,
}

impl SearchBackend {
    fn as_str(self) -> &'static str {
        match self {
            SearchBackend::DuckDuckGo => "ddg",
            SearchBackend::Mojeek => "mojeek",
        }
    }
}

#[derive(Debug, Clone)]
struct TransportClients {
    direct: reqwest::Client,
    proxy: Option<reqwest::Client>,
}

impl TransportClients {
    fn build() -> Result<Self, SearchError> {
        let direct = build_client(true)?;
        let proxy = if has_proxy_env() {
            Some(build_client(false)?)
        } else {
            None
        };

        Ok(Self { direct, proxy })
    }

    fn route_order(&self) -> [TransportRoute; 2] {
        if self.proxy.is_some() {
            [TransportRoute::Proxy, TransportRoute::Direct]
        } else {
            [TransportRoute::Direct, TransportRoute::Direct]
        }
    }

    fn client_for(&self, route: TransportRoute) -> Option<&reqwest::Client> {
        match route {
            TransportRoute::Direct => Some(&self.direct),
            TransportRoute::Proxy => self.proxy.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SafeSourcesCatalog {
    safe_sources: Vec<SafeSourcesCategory>,
}

#[derive(Debug, Clone, Deserialize)]
struct SafeSourcesCategory {
    category: String,
    sites: Vec<SafeSourceSite>,
}

#[derive(Debug, Clone, Deserialize)]
struct SafeSourceSite {
    name: String,
    domain: String,
    #[serde(default)]
    rss: Option<String>,
}

lazy_static::lazy_static! {
    static ref RESULT_LINK_RE: Regex = Regex::new(
        r#"(?is)<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#
    ).expect("valid result link regex");
    static ref RESULT_SNIPPET_RE: Regex = Regex::new(
        r#"(?is)<(?:a|div)[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</(?:a|div)>"#
    ).expect("valid result snippet regex");
    static ref TITLE_RE: Regex = Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).expect("valid title regex");
    static ref SKIP_BLOCK_RE: Regex = Regex::new(
        r#"(?is)<script[^>]*>.*?</script>|<style[^>]*>.*?</style>|<head[^>]*>.*?</head>|<noscript[^>]*>.*?</noscript>|<svg[^>]*>.*?</svg>|<math[^>]*>.*?</math>|<nav[^>]*>.*?</nav>|<footer[^>]*>.*?</footer>"#
    ).expect("valid skip block regex");
    static ref TAG_RE: Regex = Regex::new(r#"(?is)<[^>]+>"#).expect("valid tag regex");
    static ref WS_RE: Regex = Regex::new(r#"\s+"#).expect("valid whitespace regex");
    static ref MOJEEK_RESULT_BLOCK_RE: Regex = Regex::new(
        r#"(?is)<li[^>]*class="[^"]*(?:result|results-standard|serp-result)[^"]*"[^>]*>(.*?)</li>"#
    ).expect("valid mojeek result block regex");
    static ref MOJEEK_LINK_RE: Regex = Regex::new(
        r#"(?is)<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#
    ).expect("valid mojeek link regex");
    static ref MOJEEK_SNIPPET_RE: Regex = Regex::new(
        r#"(?is)<p[^>]*class="[^"]*(?:s|snippet|desc|description)[^"]*"[^>]*>(.*?)</p>"#
    ).expect("valid mojeek snippet regex");
    static ref SAFE_SOURCES_DATA: Vec<SafeSourcesCategory> = {
        let json_content = include_str!("../brain/knowledge/safe_sources.json");
        serde_json::from_str::<SafeSourcesCatalog>(json_content)
            .map(|catalog| catalog.safe_sources)
            .unwrap_or_default()
    };
    static ref SAFE_DOMAINS: HashSet<String> = {
        let mut out = HashSet::new();
        for category in SAFE_SOURCES_DATA.iter() {
            for site in &category.sites {
                out.insert(normalize_domain(&site.domain));
            }
        }
        out
    };
}

fn user_agent() -> &'static str {
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

fn encode_query(query: &str) -> String {
    url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
}

fn looks_like_loopback_host(host: &str) -> bool {
    let h = host.trim().to_ascii_lowercase();
    h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local")
}

fn is_documentation_v6(v6: &Ipv6Addr) -> bool {
    let segments = v6.segments();
    segments[0] == 0x2001 && segments[1] == 0x0db8
}

fn is_unique_local_v6(v6: &Ipv6Addr) -> bool {
    (v6.segments()[0] & 0xfe00) == 0xfc00
}

fn is_unicast_link_local_v6(v6: &Ipv6Addr) -> bool {
    (v6.segments()[0] & 0xffc0) == 0xfe80
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_private()
                && !v4.is_loopback()
                && !v4.is_link_local()
                && !v4.is_multicast()
                && !v4.is_broadcast()
                && !v4.is_documentation()
                && !v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            !v6.is_loopback()
                && !v6.is_unspecified()
                && !v6.is_multicast()
                && !is_unique_local_v6(&v6)
                && !is_unicast_link_local_v6(&v6)
                && !is_documentation_v6(&v6)
        }
    }
}

async fn ensure_public_target(url: &Url) -> Result<(), SearchError> {
    let host = url.host_str().ok_or_else(|| {
        SearchError::fatal(
            SearchFailureClass::BlockedTarget,
            "Blocked URL: missing host",
        )
    })?;
    if looks_like_loopback_host(host) {
        return Err(SearchError::fatal(
            SearchFailureClass::BlockedTarget,
            "Blocked URL: local host is not allowed",
        ));
    }

    let port = url.port_or_known_default().ok_or_else(|| {
        SearchError::fatal(
            SearchFailureClass::BlockedTarget,
            "Blocked URL: unknown port",
        )
    })?;
    let lookup = tokio::net::lookup_host((host, port)).await.map_err(|e| {
        SearchError::retriable(SearchFailureClass::Dns, format!("DNS lookup failed: {}", e))
    })?;

    let mut has_ip = false;
    for socket_addr in lookup {
        has_ip = true;
        if !is_public_ip(socket_addr.ip()) {
            return Err(SearchError::fatal(
                SearchFailureClass::BlockedTarget,
                format!("Blocked URL: non-public IP target ({})", socket_addr.ip()),
            ));
        }
    }
    if !has_ip {
        return Err(SearchError::retriable(
            SearchFailureClass::Dns,
            "DNS lookup returned no IP addresses".to_string(),
        ));
    }
    Ok(())
}

fn clean_html_fragment(input: &str) -> String {
    let mut s = input.replace("&nbsp;", " ");
    s = s.replace("&amp;", "&");
    s = s.replace("&quot;", "\"");
    s = s.replace("&#39;", "'");
    s = s.replace("&lt;", "<");
    s = s.replace("&gt;", ">");

    let without_tags = TAG_RE.replace_all(&s, " ");
    let collapsed = WS_RE.replace_all(&without_tags, " ");
    collapsed.trim().to_string()
}

fn looks_like_ddg_challenge_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("anomaly-modal")
        || lower.contains("id=\"challenge-form\"")
        || lower.contains("unfortunately, bots use duckduckgo too")
}

fn looks_like_mojeek_block_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    (lower.contains("403 - forbidden") && lower.contains("automated queries"))
        || lower.contains("captcha")
}

fn clean_page_text(raw_html: &str) -> String {
    let stripped = SKIP_BLOCK_RE.replace_all(raw_html, " ");
    let no_tags = TAG_RE.replace_all(&stripped, " ");
    let decoded = clean_html_fragment(&no_tags);
    if decoded.len() > MAX_FETCH_CHARS {
        let mut truncated = decoded[..MAX_FETCH_CHARS].to_string();
        truncated.push_str(" ...");
        return truncated;
    }
    decoded
}

fn compact_summary(text: &str, max_words: usize) -> String {
    if text.trim().is_empty() {
        return String::new();
    }
    let words: Vec<&str> = text.split_whitespace().take(max_words + 1).collect();
    if words.len() <= max_words {
        words.join(" ")
    } else {
        format!("{}...", words[..max_words].join(" "))
    }
}

fn extract_title(raw_html: &str, fallback_url: &str) -> String {
    if let Some(cap) = TITLE_RE.captures(raw_html) {
        if let Some(m) = cap.get(1) {
            let title = clean_html_fragment(m.as_str());
            if !title.is_empty() {
                return title;
            }
        }
    }
    Url::parse(fallback_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| fallback_url.to_string())
}

fn canonicalize_url(raw: &str) -> Result<String, SearchError> {
    let mut input = raw.trim().to_string();
    if input.is_empty() {
        return Err(SearchError::fatal(
            SearchFailureClass::InvalidUrl,
            "Invalid URL: empty",
        ));
    }

    if input.starts_with("//") {
        input = format!("https:{}", input);
    }
    if input.starts_with("/l/?") {
        input = format!("https://duckduckgo.com{}", input);
    }

    let mut parsed = Url::parse(&input).map_err(|_| {
        SearchError::fatal(
            SearchFailureClass::InvalidUrl,
            format!("Invalid URL: {}", raw.trim()),
        )
    })?;

    if parsed.host_str() == Some("duckduckgo.com") && parsed.path().starts_with("/l/") {
        let mut uddg_value = None;
        for (k, v) in parsed.query_pairs() {
            if k == "uddg" {
                uddg_value = Some(v.to_string());
                break;
            }
        }
        if let Some(decoded) = uddg_value {
            parsed = Url::parse(&decoded).map_err(|_| {
                SearchError::fatal(
                    SearchFailureClass::InvalidUrl,
                    format!("Invalid redirect URL: {}", decoded),
                )
            })?;
        }
    }

    let scheme = parsed.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(SearchError::fatal(
            SearchFailureClass::InvalidUrl,
            format!("Blocked URL scheme: {}", parsed.scheme()),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(SearchError::fatal(
            SearchFailureClass::InvalidUrl,
            "Blocked URL: host is required",
        ));
    }

    parsed.set_fragment(None);
    if (parsed.scheme() == "https" && parsed.port() == Some(443))
        || (parsed.scheme() == "http" && parsed.port() == Some(80))
    {
        let _ = parsed.set_port(None);
    }

    Ok(parsed.to_string())
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

async fn send_with_transport<F>(
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

async fn read_capped_response_body(
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

async fn fetch_html_with_redirects(
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

fn parse_ddg_results(html: &str, max_results: usize) -> Vec<CitationSource> {
    let mut sources = Vec::<CitationSource>::new();
    let mut seen = HashSet::<String>::new();

    let link_matches: Vec<_> = RESULT_LINK_RE.find_iter(html).collect();
    for (idx, m) in link_matches.iter().enumerate() {
        if sources.len() >= max_results {
            break;
        }

        let Some(cap) = RESULT_LINK_RE.captures(m.as_str()) else {
            continue;
        };
        let Some(url_raw) = cap.get(1).map(|v| v.as_str()) else {
            continue;
        };
        let Some(title_raw) = cap.get(2).map(|v| v.as_str()) else {
            continue;
        };

        let canonical = match canonicalize_url(url_raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !seen.insert(canonical.clone()) {
            continue;
        }

        let segment_start = m.end();
        let segment_end = link_matches
            .get(idx + 1)
            .map(|n| n.start())
            .unwrap_or(html.len());
        let segment = &html[segment_start..segment_end.min(html.len())];

        let snippet = RESULT_SNIPPET_RE
            .captures(segment)
            .and_then(|c| c.get(1).or_else(|| c.get(2)))
            .map(|v| clean_html_fragment(v.as_str()))
            .unwrap_or_default();

        let title = clean_html_fragment(title_raw);
        sources.push(citation_source(
            if title.is_empty() {
                Url::parse(&canonical)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_string()))
                    .unwrap_or_else(|| canonical.clone())
            } else {
                title
            },
            canonical,
            compact_summary(&snippet, MAX_SUMMARY_WORDS),
        ));
    }

    sources
}

fn parse_mojeek_results(html: &str, max_results: usize) -> Vec<CitationSource> {
    let mut sources = Vec::<CitationSource>::new();
    let mut seen = HashSet::<String>::new();

    for block in MOJEEK_RESULT_BLOCK_RE.captures_iter(html) {
        if sources.len() >= max_results {
            break;
        }

        let Some(block_html) = block.get(1).map(|m| m.as_str()) else {
            continue;
        };

        let Some(link_cap) = MOJEEK_LINK_RE.captures(block_html) else {
            continue;
        };

        let Some(url_raw) = link_cap.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let Some(title_raw) = link_cap.get(2).map(|m| m.as_str()) else {
            continue;
        };

        let canonical = match canonicalize_url(url_raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(domain) = domain_from_url(&canonical) {
            if domain.ends_with("mojeek.com") {
                continue;
            }
        }

        if !seen.insert(canonical.clone()) {
            continue;
        }

        let snippet = MOJEEK_SNIPPET_RE
            .captures(block_html)
            .and_then(|c| c.get(1))
            .map(|v| clean_html_fragment(v.as_str()))
            .unwrap_or_default();

        let title = clean_html_fragment(title_raw);
        sources.push(citation_source(
            if title.is_empty() {
                Url::parse(&canonical)
                    .ok()
                    .and_then(|u| u.host_str().map(|h| h.to_string()))
                    .unwrap_or_else(|| canonical.clone())
            } else {
                title
            },
            canonical,
            compact_summary(&snippet, MAX_SUMMARY_WORDS),
        ));
    }

    if !sources.is_empty() {
        return sources;
    }

    // Fallback parser for looser markup.
    for cap in MOJEEK_LINK_RE.captures_iter(html) {
        if sources.len() >= max_results {
            break;
        }

        let Some(url_raw) = cap.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let Some(title_raw) = cap.get(2).map(|m| m.as_str()) else {
            continue;
        };

        if !url_raw.starts_with("http://") && !url_raw.starts_with("https://") {
            continue;
        }

        let canonical = match canonicalize_url(url_raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(domain) = domain_from_url(&canonical) {
            if domain.ends_with("mojeek.com") {
                continue;
            }
        }

        if !seen.insert(canonical.clone()) {
            continue;
        }

        let title = clean_html_fragment(title_raw);
        if title.len() < 8 {
            continue;
        }

        sources.push(citation_source(title, canonical, String::new()));
    }

    sources
}

fn source_score(query: &str, source: &CitationSource) -> i32 {
    let mut score = 0i32;

    if let Some(domain) = domain_from_url(&source.url) {
        if is_safe_domain(&domain) {
            score += 30;
        }
    }

    if source.summary.is_empty() {
        score -= 3;
    } else {
        score += 4;
    }

    if let Ok(url) = Url::parse(&source.url) {
        let path = url.path().trim_matches('/');
        if path.is_empty() {
            score -= 8;
        } else {
            score += 2;
        }
    }

    let q = query.to_ascii_lowercase();
    let title = source.title.to_ascii_lowercase();
    if !q.is_empty() && q.split_whitespace().any(|token| title.contains(token)) {
        score += 2;
    }

    score
}

fn rerank_sources(
    query: &str,
    mut sources: Vec<CitationSource>,
    max_results: usize,
) -> Vec<CitationSource> {
    let mut indexed: Vec<(usize, CitationSource, i32)> = sources
        .drain(..)
        .enumerate()
        .map(|(idx, source)| {
            let score = source_score(query, &source);
            (idx, source, score)
        })
        .collect();

    indexed.sort_by_key(|(idx, _, score)| (Reverse(*score), *idx));
    indexed
        .into_iter()
        .take(max_results)
        .map(|(_, source, _)| source)
        .collect()
}

fn build_query_context(query: &str, sources: &[CitationSource]) -> String {
    let mut context = format!("[Search results for \"{}\"]\n", query.trim());
    for source in sources {
        context.push_str(&format!(
            "- {} — {}\n  {}\n",
            source.title,
            source.url,
            if source.summary.is_empty() {
                "(No snippet available)"
            } else {
                &source.summary
            }
        ));
    }
    context.trim().to_string()
}

pub fn build_query_result(
    query: &str,
    sources: Vec<CitationSource>,
    message: Option<String>,
) -> WebSearchResult {
    WebSearchResult {
        mode: "query".to_string(),
        query: Some(query.trim().to_string()),
        requested_url: None,
        context_markdown: build_query_context(query, &sources),
        sources,
        success: true,
        message,
    }
}

async fn run_backend_query_once(
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

async fn run_url_fetch_once(
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

fn emit_progress(
    progress: &mut Option<&mut (dyn FnMut(String) + Send)>,
    message: impl Into<String>,
) {
    if let Some(cb) = progress.as_deref_mut() {
        cb(message.into());
    }
}

async fn with_retries_with_progress<T, F, Fut>(
    _label: &str,
    mut op: F,
    _progress: &mut Option<&mut (dyn FnMut(String) + Send)>,
) -> Result<T, SearchError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, SearchError>>,
{
    let mut attempt = 0usize;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if !e.retriable || attempt >= MAX_RETRIES {
                    return Err(e);
                }
                attempt += 1;
                let exp = 2u64.pow(attempt as u32);
                let jitter = rand::thread_rng().gen_range(80..220);
                let wait_ms = exp * 220 + jitter;
                sleep(Duration::from_millis(wait_ms)).await;
            }
        }
    }
}

fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_start_matches('.').to_ascii_lowercase()
}

pub fn domain_from_url(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(normalize_domain))
        .map(|d| d.trim_start_matches("www.").to_string())
}

fn favicon_for_url(url: &str) -> Option<String> {
    domain_from_url(url)
        .map(|domain| format!("https://www.google.com/s2/favicons?domain={}&sz=32", domain))
}

fn active_chat_storage() -> Option<ChatStorage> {
    let profile_store = ProfileStore::new().ok()?;
    let active_id = profile_store.get_active_profile_id().ok()??;
    let chats_dir = profile_store.get_chats_dir(&active_id);
    ChatStorage::with_base_dir(chats_dir).ok()
}

fn is_remote_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn favicon_extension_from_content_type(content_type: &str) -> Option<&'static str> {
    let normalized = content_type
        .split(';')
        .next()
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();

    match normalized.as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        _ => None,
    }
}

fn favicon_extension_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let ext = Path::new(parsed.path())
        .extension()
        .and_then(|v| v.to_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())?
        .to_ascii_lowercase();

    if ext.len() > 8 || !ext.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }

    Some(ext)
}

async fn fetch_and_store_favicon(
    client: reqwest::Client,
    storage: Arc<ChatStorage>,
    favicon_url: String,
) -> (String, Option<String>) {
    let response = match client
        .get(&favicon_url)
        .header(header::ACCEPT, "image/*,*/*;q=0.8")
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return (favicon_url, None),
    };

    if !response.status().is_success() {
        return (favicon_url, None);
    }

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());

    if let Some(kind) = content_type.as_deref() {
        let normalized = kind
            .split(';')
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !normalized.starts_with("image/") && normalized != "application/octet-stream" {
            return (favicon_url, None);
        }
    }

    let bytes = match response.bytes().await {
        Ok(body) => body,
        Err(_) => return (favicon_url, None),
    };

    if bytes.is_empty() || bytes.len() > MAX_FAVICON_BYTES {
        return (favicon_url, None);
    }

    let extension = content_type
        .as_deref()
        .and_then(favicon_extension_from_content_type)
        .map(str::to_string)
        .or_else(|| favicon_extension_from_url(&favicon_url))
        .unwrap_or_else(|| "png".to_string());

    match storage.store_file(bytes.as_ref(), &extension, None) {
        Ok(stored) => (favicon_url, Some(stored.path)),
        Err(_) => (favicon_url, None),
    }
}

async fn cache_favicons_for_sources(sources: &mut [CitationSource]) {
    if sources.is_empty() {
        return;
    }

    let Some(storage) = active_chat_storage() else {
        return;
    };
    let storage = Arc::new(storage);

    let favicon_client = match reqwest::Client::builder()
        .redirect(Policy::limited(MAX_REDIRECTS))
        .timeout(Duration::from_secs(FAVICON_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(FAVICON_CONNECT_TIMEOUT_SECS))
        .user_agent(user_agent())
        .build()
    {
        Ok(client) => client,
        Err(_) => return,
    };

    let mut unique_urls = Vec::<String>::new();
    let mut seen_urls = HashSet::<String>::new();

    for source in sources.iter() {
        let Some(favicon_url) = source
            .favicon
            .as_deref()
            .map(str::trim)
            .filter(|v| is_remote_http_url(v))
        else {
            continue;
        };

        if seen_urls.insert(favicon_url.to_string()) {
            unique_urls.push(favicon_url.to_string());
        }
    }

    if unique_urls.is_empty() {
        return;
    }

    let tasks = unique_urls.into_iter().map(|favicon_url| {
        fetch_and_store_favicon(favicon_client.clone(), Arc::clone(&storage), favicon_url)
    });
    let cached_favicon_paths = join_all(tasks).await.into_iter().collect::<HashMap<_, _>>();

    for source in sources.iter_mut() {
        let Some(favicon_url) = source
            .favicon
            .as_deref()
            .map(str::trim)
            .filter(|v| is_remote_http_url(v))
        else {
            continue;
        };

        if let Some(Some(local_path)) = cached_favicon_paths.get(favicon_url) {
            source.favicon = Some(local_path.clone());
        }
    }
}

fn citation_source(title: String, url: String, summary: String) -> CitationSource {
    let favicon = favicon_for_url(&url);
    CitationSource {
        title,
        url,
        summary,
        favicon,
    }
}

fn is_safe_domain(domain: &str) -> bool {
    let normalized = normalize_domain(domain)
        .trim_start_matches("www.")
        .to_string();
    SAFE_DOMAINS
        .iter()
        .any(|allowed| normalized == *allowed || normalized.ends_with(&format!(".{}", allowed)))
}

fn make_safe_source_url(site: &SafeSourceSite) -> Option<String> {
    let preferred = site
        .rss
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .unwrap_or_else(|| format!("https://{}/", site.domain.trim()));

    canonicalize_url(&preferred).ok()
}

fn query_terms(query: &str) -> HashSet<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .map(|part| part.trim().to_ascii_lowercase())
        .filter(|part| part.len() >= 2)
        .collect()
}

fn safe_source_relevance_score(query: &str, category: &str, site: &SafeSourceSite) -> i32 {
    let terms = query_terms(query);
    if terms.is_empty() {
        return 0;
    }

    let haystack = format!(
        "{} {} {}",
        category.to_ascii_lowercase(),
        site.name.to_ascii_lowercase(),
        site.domain.to_ascii_lowercase()
    );

    terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count() as i32
}

pub fn local_safe_source_candidates(
    query: &str,
    attempted_domains: &HashSet<String>,
    max_candidates: usize,
) -> Vec<CitationSource> {
    if max_candidates == 0 {
        return Vec::new();
    }

    let mut candidates = Vec::<(usize, i32, String, SafeSourceSite)>::new();
    let mut seen_domains = HashSet::<String>::new();
    let mut ordinal = 0usize;

    for category in SAFE_SOURCES_DATA.iter() {
        let category_name = category.category.to_ascii_lowercase();
        for site in &category.sites {
            let domain = normalize_domain(&site.domain)
                .trim_start_matches("www.")
                .to_string();

            if attempted_domains.contains(&domain) || !seen_domains.insert(domain.clone()) {
                continue;
            }

            let score = safe_source_relevance_score(query, &category_name, site);
            candidates.push((ordinal, score, category_name.clone(), site.clone()));
            ordinal += 1;
        }
    }

    candidates.sort_by_key(|(idx, score, _, _)| (Reverse(*score), *idx));

    let mut out = Vec::<CitationSource>::new();
    for (_, _, category, site) in candidates.into_iter().take(max_candidates) {
        let Some(url) = make_safe_source_url(&site) else {
            continue;
        };
        out.push(citation_source(
            site.name,
            url,
            format!("Trusted {} source candidate.", category),
        ));
    }

    out
}

pub fn filter_suggested_urls_to_safe_sources(
    urls: &[String],
    attempted_domains: &HashSet<String>,
    max_candidates: usize,
) -> Vec<CitationSource> {
    if max_candidates == 0 {
        return Vec::new();
    }

    let mut out = Vec::<CitationSource>::new();
    let mut seen_domains = HashSet::<String>::new();
    let mut seen_urls = HashSet::<String>::new();

    for url in urls {
        if out.len() >= max_candidates {
            break;
        }

        let Ok(canonical) = canonicalize_url(url) else {
            continue;
        };
        let Some(domain) = domain_from_url(&canonical) else {
            continue;
        };

        if !is_safe_domain(&domain)
            || attempted_domains.contains(&domain)
            || !seen_domains.insert(domain.clone())
            || !seen_urls.insert(canonical.clone())
        {
            continue;
        }

        out.push(citation_source(
            domain.clone(),
            canonical,
            "Gemini-assisted trusted source candidate.".to_string(),
        ));
    }

    out
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

pub fn collect_allowed_sources(result: &WebSearchResult) -> HashMap<String, CitationSource> {
    let mut out = HashMap::<String, CitationSource>::new();
    for source in &result.sources {
        if let Ok(canonical) = canonicalize_url(&source.url) {
            out.insert(canonical, source.clone());
        }
    }
    out
}

fn build_degraded_url_result(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_ddg_redirect_url() {
        let input = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa%2Fb%3Fx%3D1";
        let out = canonicalize_url(input).expect("url should parse");
        assert!(out.starts_with("https://example.com/a/b?x=1"));
    }

    #[test]
    fn canonicalize_blocks_non_http() {
        let out = canonicalize_url("file:///etc/passwd");
        assert!(out.is_err());
    }

    #[test]
    fn compact_summary_truncates() {
        let text = "one two three four five six";
        let out = compact_summary(text, 4);
        assert_eq!(out, "one two three four...");
    }

    #[test]
    fn ddg_challenge_detection_works() {
        let html = "<div id=\"challenge-form\">Unfortunately, bots use DuckDuckGo too</div>";
        assert!(looks_like_ddg_challenge_page(html));
    }

    #[test]
    fn mojeek_block_detection_works() {
        let html = "<title>403 - Forbidden</title> Sorry your network appears to be sending automated queries";
        assert!(looks_like_mojeek_block_page(html));
    }

    #[test]
    fn parses_mojeek_results_fixture() {
        let html = r#"
        <ul>
          <li class="results-standard">
            <h2><a href="https://example.com/post">Example Post</a></h2>
            <p class="s">Example snippet from fixture.</p>
          </li>
          <li class="results-standard">
            <h2><a href="https://docs.rust-lang.org/book/">Rust Book</a></h2>
            <p class="description">Learn Rust from official docs.</p>
          </li>
        </ul>
        "#;

        let out = parse_mojeek_results(html, 5);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].title, "Example Post");
        assert!(out[0].url.starts_with("https://example.com/post"));
    }

    #[test]
    fn safe_source_candidates_respect_attempted_domains() {
        let mut attempted = HashSet::new();
        attempted.insert("news.ycombinator.com".to_string());

        let candidates = local_safe_source_candidates("latest ai models", &attempted, 5);
        assert!(!candidates.is_empty());
        for c in candidates {
            let domain = domain_from_url(&c.url).expect("domain expected");
            assert_ne!(domain, "news.ycombinator.com");
        }
    }

    #[test]
    fn filter_suggested_urls_keeps_only_safe_domains() {
        let attempted = HashSet::new();
        let urls = vec![
            "https://news.ycombinator.com/item?id=123".to_string(),
            "https://example.org/unsafe".to_string(),
        ];
        let out = filter_suggested_urls_to_safe_sources(&urls, &attempted, 5);

        assert_eq!(out.len(), 1);
        assert!(out[0].url.contains("news.ycombinator.com"));
    }

    #[test]
    fn collect_allowed_sources_canonicalizes_and_maps() {
        let result = WebSearchResult {
            mode: "query".to_string(),
            query: Some("test".to_string()),
            requested_url: None,
            context_markdown: String::new(),
            success: true,
            message: None,
            sources: vec![CitationSource {
                title: "Example".to_string(),
                url: "https://example.com:443/path#section".to_string(),
                summary: "Summary".to_string(),
                favicon: None,
            }],
        };

        let allowed = collect_allowed_sources(&result);
        assert!(allowed.contains_key("https://example.com/path"));
    }
}
