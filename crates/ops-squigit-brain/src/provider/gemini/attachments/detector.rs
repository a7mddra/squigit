// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use std::collections::{HashMap, HashSet};

fn unwrap_link_destination(path: &str) -> &str {
    let trimmed = path.trim();
    trimmed
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
        .map(str::trim)
        .unwrap_or(trimmed)
}

fn is_attachment_link_path(path: &str) -> bool {
    let value = unwrap_link_destination(path);
    if value.is_empty() {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
    {
        return false;
    }

    if value.starts_with('/') || value.starts_with("\\\\") {
        return true;
    }

    let bytes = value.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }

    value.starts_with("objects/")
        || value.starts_with("./objects/")
        || value.starts_with("../objects/")
        || value.starts_with("tmp/")
        || value.starts_with("/tmp/")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachmentMention {
    pub path: String,
    pub display_name: Option<String>,
}

fn normalize_display_name(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
}

/// Extract attachment-like local mentions from markdown links, legacy `{{path}}` tokens,
/// and `[Attachment References]` blocks (`` - `name`: `path` ``).
/// Returns deduplicated mentions while preserving first-seen order.
pub(crate) fn extract_attachment_mentions(text: &str) -> Vec<AttachmentMention> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let re = Regex::new(
        r"(?x)
        (\{\{(?P<legacy_path>[^}]+)\}\})
        |
        (\[(?P<link_label>[^\]\n]+)\]\((?P<link_path><[^>\n]+>|[^)\n]+)\))
        |
        (-\s+`(?P<ref_label>[^`]+)`:\s+`(?P<ref_path>[^`]+)`)
    ",
    )
    .expect("Attachment detector regex must compile");

    let mut mentions = Vec::<AttachmentMention>::new();
    let mut path_to_index = HashMap::<String, usize>::new();

    for cap in re.captures_iter(text) {
        let mention = if let Some(legacy) = cap.name("legacy_path") {
            let path = legacy.as_str().trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(AttachmentMention {
                    path,
                    display_name: None,
                })
            }
        } else if let Some(link_path) = cap.name("link_path") {
            let path = unwrap_link_destination(link_path.as_str())
                .trim()
                .to_string();
            if !is_attachment_link_path(&path) || path.is_empty() {
                None
            } else {
                let display_name = cap
                    .name("link_label")
                    .and_then(|label| normalize_display_name(label.as_str()));
                Some(AttachmentMention { path, display_name })
            }
        } else if let Some(ref_path) = cap.name("ref_path") {
            let path = ref_path.as_str().trim().to_string();
            if !is_attachment_link_path(&path) || path.is_empty() {
                None
            } else {
                let display_name = cap
                    .name("ref_label")
                    .and_then(|label| normalize_display_name(label.as_str()));
                Some(AttachmentMention { path, display_name })
            }
        } else {
            None
        };

        let Some(mention) = mention else {
            continue;
        };

        if let Some(index) = path_to_index.get(&mention.path).copied() {
            // Preserve first-seen order, but keep the first available display name.
            if mentions[index].display_name.is_none() && mention.display_name.is_some() {
                mentions[index].display_name = mention.display_name;
            }
            continue;
        }

        path_to_index.insert(mention.path.clone(), mentions.len());
        mentions.push(mention);
    }

    mentions
}

/// Extract attachment-like local paths from markdown links and legacy `{{path}}` tokens.
/// Returns deduplicated paths while preserving first-seen order.
#[allow(dead_code)]
pub(crate) fn extract_attachment_paths(text: &str) -> Vec<String> {
    let mentions = extract_attachment_mentions(text);
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for mention in mentions {
        if seen.insert(mention.path.clone()) {
            paths.push(mention.path);
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_markdown_attachment_links() {
        let text = "Please check [file](objects/ab/hash.md) and [pdf](/tmp/a.pdf).";
        let out = extract_attachment_paths(text);
        assert_eq!(out, vec!["objects/ab/hash.md", "/tmp/a.pdf"]);
    }

    #[test]
    fn extracts_legacy_tokens() {
        let text = "Analyze {{objects/aa/abcdef1234.rs}} now";
        let out = extract_attachment_paths(text);
        assert_eq!(out, vec!["objects/aa/abcdef1234.rs"]);
    }

    #[test]
    fn dedupes_and_preserves_order() {
        let text = "[a](objects/a/file.txt) [b](objects/b/file.txt) [c](objects/a/file.txt)";
        let out = extract_attachment_paths(text);
        assert_eq!(out, vec!["objects/a/file.txt", "objects/b/file.txt"]);
    }

    #[test]
    fn excludes_non_attachment_links() {
        let text = "[site](https://example.com) [mail](mailto:test@example.com)";
        let out = extract_attachment_paths(text);
        assert!(out.is_empty());
    }

    #[test]
    fn unwraps_angle_brackets_in_links() {
        let text = "[doc](<objects/ab/hash.docx>)";
        let out = extract_attachment_paths(text);
        assert_eq!(out, vec!["objects/ab/hash.docx"]);
    }

    #[test]
    fn extracts_display_name_from_markdown_label() {
        let text = "[Quarterly Report.pdf](objects/ab/hash.pdf)";
        let mentions = extract_attachment_mentions(text);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].path, "objects/ab/hash.pdf");
        assert_eq!(
            mentions[0].display_name.as_deref(),
            Some("Quarterly Report.pdf")
        );
    }

    #[test]
    fn dedupe_keeps_first_available_display_name() {
        let text = "{{objects/ab/hash.pdf}} [Human Name](objects/ab/hash.pdf)";
        let mentions = extract_attachment_mentions(text);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].path, "objects/ab/hash.pdf");
        assert_eq!(mentions[0].display_name.as_deref(), Some("Human Name"));
    }

    #[test]
    fn extracts_from_attachment_references_block() {
        let text = "[Attachment References]\n- `build.rs`: `objects/ab/hash.rs`";
        let mentions = extract_attachment_mentions(text);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].path, "objects/ab/hash.rs");
        assert_eq!(mentions[0].display_name.as_deref(), Some("build.rs"));
    }

    #[test]
    fn extracts_multiple_from_attachment_references_block() {
        let text = "[Attachment References]\n- `main.rs`: `objects/ab/aaa.rs`\n- `lib.rs`: `objects/cd/bbb.rs`";
        let mentions = extract_attachment_mentions(text);
        assert_eq!(mentions.len(), 2);
        assert_eq!(mentions[0].path, "objects/ab/aaa.rs");
        assert_eq!(mentions[0].display_name.as_deref(), Some("main.rs"));
        assert_eq!(mentions[1].path, "objects/cd/bbb.rs");
        assert_eq!(mentions[1].display_name.as_deref(), Some("lib.rs"));
    }

    #[test]
    fn dedupe_across_link_and_reference_formats() {
        let text = "[build.rs](objects/ab/hash.rs)\n\n[Attachment References]\n- `build.rs`: `objects/ab/hash.rs`";
        let mentions = extract_attachment_mentions(text);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].path, "objects/ab/hash.rs");
        assert_eq!(mentions[0].display_name.as_deref(), Some("build.rs"));
    }
}
