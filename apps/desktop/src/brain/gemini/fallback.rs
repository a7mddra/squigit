// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::time::Duration;

use super::types::{GeminiContent, GeminiPart, GeminiRequest, GeminiResponseChunk};

fn extract_text_from_non_stream_chunk(chunk: &GeminiResponseChunk) -> Option<String> {
    chunk.candidates.as_ref().and_then(|candidates| {
        candidates.first().and_then(|first| {
            first.content.as_ref().and_then(|content| {
                content.parts.as_ref().and_then(|parts| {
                    parts
                        .iter()
                        .find_map(|part| part.text.as_ref().map(|s| s.trim().to_string()))
                        .filter(|s| !s.is_empty())
                })
            })
        })
    })
}

fn parse_url_array_from_text(raw: &str, max_urls: usize) -> Vec<String> {
    fn normalize_urls(values: Vec<String>, max_urls: usize) -> Vec<String> {
        let mut out = Vec::<String>::new();
        let mut seen = HashSet::<String>::new();
        for value in values {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(parsed) = url::Url::parse(trimmed) else {
                continue;
            };
            let scheme = parsed.scheme().to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                continue;
            }
            let normalized = parsed.to_string();
            if seen.insert(normalized.clone()) {
                out.push(normalized);
                if out.len() >= max_urls {
                    break;
                }
            }
        }
        out
    }

    let text = raw.trim();
    if text.is_empty() {
        return Vec::new();
    }

    if let Ok(values) = serde_json::from_str::<Vec<String>>(text) {
        return normalize_urls(values, max_urls);
    }

    let start = text.find('[');
    let end = text.rfind(']');
    if let (Some(s), Some(e)) = (start, end) {
        if s < e {
            let slice = &text[s..=e];
            if let Ok(values) = serde_json::from_str::<Vec<String>>(slice) {
                return normalize_urls(values, max_urls);
            }
        }
    }

    Vec::new()
}

pub(crate) async fn suggest_fallback_urls(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    query: &str,
    max_urls: usize,
) -> Vec<String> {
    let suggest_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let prompt = format!(
        "Suggest up to {max_urls} direct, publicly accessible URLs for this query: \"{query}\".\n\
         Prefer captcha-free sources like Wikipedia, official docs, blogs, and trusted public pages.\n\
         Return ONLY a JSON array of URLs. No markdown, no explanation."
    );

    let request_body = GeminiRequest {
        system_instruction: None,
        contents: vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: Some(prompt),
                ..Default::default()
            }],
        }],
        generation_config: None,
        tools: None,
        tool_config: None,
    };

    let response = match tokio::time::timeout(
        Duration::from_secs(8),
        client.post(&suggest_url).json(&request_body).send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp,
        Ok(Err(_)) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    if !response.status().is_success() {
        return Vec::new();
    }

    let body = match tokio::time::timeout(Duration::from_secs(8), response.text()).await {
        Ok(Ok(v)) => v,
        Ok(Err(_)) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    let chunk: GeminiResponseChunk = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let Some(text) = extract_text_from_non_stream_chunk(&chunk) else {
        return Vec::new();
    };

    parse_url_array_from_text(&text, max_urls)
}
