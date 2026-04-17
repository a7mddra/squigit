// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::future::join_all;
use ops_chat_storage::ChatStorage;
use ops_profile_store::ProfileStore;
use reqwest::{header, redirect::Policy};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use super::constants::{
    FAVICON_CONNECT_TIMEOUT_SECS, FAVICON_TIMEOUT_SECS, MAX_FAVICON_BYTES, MAX_REDIRECTS,
};
use super::transport::user_agent;
use super::types::CitationSource;
use super::url_utils::{domain_from_url, is_remote_http_url};

pub(crate) fn favicon_for_url(url: &str) -> Option<String> {
    domain_from_url(url)
        .map(|domain| format!("https://www.google.com/s2/favicons?domain={}&sz=32", domain))
}

fn active_chat_storage() -> Option<ChatStorage> {
    let profile_store = ProfileStore::new().ok()?;
    let active_id = profile_store.get_active_profile_id().ok()??;
    let chats_dir = profile_store.get_chats_dir(&active_id);
    ChatStorage::with_base_dir(chats_dir).ok()
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

pub(crate) async fn cache_favicons_for_sources(sources: &mut [CitationSource]) {
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

pub(crate) fn citation_source(title: String, url: String, summary: String) -> CitationSource {
    let favicon = favicon_for_url(&url);
    CitationSource {
        title,
        url,
        summary,
        favicon,
    }
}
