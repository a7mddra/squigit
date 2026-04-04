// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use std::cmp::Reverse;
use std::collections::HashSet;
use url::Url;

use super::constants::{MAX_FETCH_CHARS, MAX_SUMMARY_WORDS};
use super::favicon::citation_source;
use super::types::{CitationSource, WebSearchResult};
use super::url_utils::{canonicalize_url, domain_from_url};

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

pub(crate) fn looks_like_ddg_challenge_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    lower.contains("anomaly-modal")
        || lower.contains("id=\"challenge-form\"")
        || lower.contains("unfortunately, bots use duckduckgo too")
}

pub(crate) fn looks_like_mojeek_block_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    (lower.contains("403 - forbidden") && lower.contains("automated queries"))
        || lower.contains("captcha")
}

pub(crate) fn clean_page_text(raw_html: &str) -> String {
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

pub(crate) fn compact_summary(text: &str, max_words: usize) -> String {
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

pub(crate) fn extract_title(raw_html: &str, fallback_url: &str) -> String {
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

pub(crate) fn parse_ddg_results(html: &str, max_results: usize) -> Vec<CitationSource> {
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

pub(crate) fn parse_mojeek_results(html: &str, max_results: usize) -> Vec<CitationSource> {
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
        if super::safe_sources::is_safe_domain(&domain) {
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

pub(crate) fn rerank_sources(
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
