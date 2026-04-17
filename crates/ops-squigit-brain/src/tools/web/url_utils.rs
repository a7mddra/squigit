// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv6Addr};
use url::Url;

use super::types::{SearchError, SearchFailureClass};

pub(crate) fn encode_query(query: &str) -> String {
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

pub(crate) async fn ensure_public_target(url: &Url) -> Result<(), SearchError> {
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

pub(crate) fn normalize_domain(domain: &str) -> String {
    domain.trim().trim_start_matches('.').to_ascii_lowercase()
}

pub fn domain_from_url(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(normalize_domain))
        .map(|d| d.trim_start_matches("www.").to_string())
}

pub(crate) fn canonicalize_url(raw: &str) -> Result<String, SearchError> {
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

pub(crate) fn is_remote_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
