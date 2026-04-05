// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

mod constants;
mod favicon;
mod fetch;
mod html;
mod query;
mod retry;
mod safe_sources;
mod suggester;
mod transport;
mod types;
mod url_utils;

pub use fetch::{
    collect_allowed_sources, fetch_url_from_allowed, fetch_url_from_allowed_with_progress,
};
pub use html::build_query_result;
pub use query::{search_query, search_query_with_progress};
pub use safe_sources::{filter_suggested_urls_to_safe_sources, local_safe_source_candidates};
pub use types::{CitationSource, WebSearchResult};
pub use url_utils::domain_from_url;
pub(crate) use suggester::suggest_fallback_urls;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn canonicalize_ddg_redirect_url() {
        let input = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fa%2Fb%3Fx%3D1";
        let out = url_utils::canonicalize_url(input).expect("url should parse");
        assert!(out.starts_with("https://example.com/a/b?x=1"));
    }

    #[test]
    fn canonicalize_blocks_non_http() {
        let out = url_utils::canonicalize_url("file:///etc/passwd");
        assert!(out.is_err());
    }

    #[test]
    fn compact_summary_truncates() {
        let text = "one two three four five six";
        let out = html::compact_summary(text, 4);
        assert_eq!(out, "one two three four...");
    }

    #[test]
    fn ddg_challenge_detection_works() {
        let html = "<div id=\"challenge-form\">Unfortunately, bots use DuckDuckGo too</div>";
        assert!(html::looks_like_ddg_challenge_page(html));
    }

    #[test]
    fn mojeek_block_detection_works() {
        let html = "<title>403 - Forbidden</title> Sorry your network appears to be sending automated queries";
        assert!(html::looks_like_mojeek_block_page(html));
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

        let out = html::parse_mojeek_results(html, 5);
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
