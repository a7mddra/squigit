// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::context::loader::{interpolate, load_attachment_preview_context};
use crate::provider::gemini::attachments::{is_gemini_document_path, is_text_like_path};

pub(crate) const PREVIEW_MAX_FILES: usize = 6;
pub(crate) const PREVIEW_SOFT_MAX_CHARS_PER_FILE: usize = 2_000;
pub(crate) const PREVIEW_HARD_MAX_TOTAL_CHARS: usize = 12_000;
pub(crate) const LOCAL_TOOL_DEFAULT_MAX_CHARS: usize = 30_000;
pub(crate) const LOCAL_TOOL_MAX_CHARS: usize = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LocalAttachmentContextSuccess {
    pub ok: bool,
    pub path: String,
    pub kind: String,
    pub text: String,
    pub truncated: bool,
    pub char_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LocalAttachmentContextFailure {
    pub ok: bool,
    pub path: String,
    pub error_code: String,
    pub error_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum LocalAttachmentContextResult {
    Success(LocalAttachmentContextSuccess),
    Failure(LocalAttachmentContextFailure),
}

impl LocalAttachmentContextResult {
    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({
                "ok": false,
                "path": "",
                "error_code": "serialization_failure",
                "error_message": "Failed to serialize local attachment context result"
            })
        })
    }
}

pub(crate) fn clamp_tool_max_chars(requested: Option<usize>) -> usize {
    requested
        .unwrap_or(LOCAL_TOOL_DEFAULT_MAX_CHARS)
        .clamp(1, LOCAL_TOOL_MAX_CHARS)
}

fn truncate_to_char_limit(text: &str, max_chars: usize) -> (String, bool) {
    if max_chars == 0 {
        return (String::new(), !text.is_empty());
    }

    let total_chars = text.chars().count();
    if total_chars <= max_chars {
        return (text.to_string(), false);
    }

    let mut out = String::new();
    for ch in text.chars().take(max_chars) {
        out.push(ch);
    }

    (out, true)
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn parse_kind(path: &Path) -> Result<&'static str, String> {
    let raw = path.to_string_lossy();

    if is_text_like_path(&raw) {
        return Ok("text");
    }

    if is_gemini_document_path(&raw) {
        return Err(
            "This document type is not supported by the local context reader. It is attached via Gemini Files."
                .to_string(),
        );
    }

    let ext = extension_lower(path);

    Err(format!("Unsupported file type: .{}", ext))
}

fn resolution_error_code(message: &str) -> &'static str {
    if message.contains("outside active chat storage scope") {
        return "path_out_of_scope";
    }
    if message.contains("Attachment not found") {
        return "attachment_not_found";
    }

    "path_resolution_failed"
}

async fn read_text_lossy(path: &Path) -> Result<String, String> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| format!("Failed to read text file: {}", e))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn empty_text_error(path: &str, kind: &str) -> LocalAttachmentContextResult {
    LocalAttachmentContextResult::Failure(LocalAttachmentContextFailure {
        ok: false,
        path: path.to_string(),
        error_code: "empty_content".to_string(),
        error_message: format!("No readable {} content was extracted", kind),
    })
}

pub(crate) async fn read_local_attachment_context(
    path: &str,
    max_chars: Option<usize>,
) -> LocalAttachmentContextResult {
    let trimmed_path = path.trim();
    if trimmed_path.is_empty() {
        return LocalAttachmentContextResult::Failure(LocalAttachmentContextFailure {
            ok: false,
            path: String::new(),
            error_code: "invalid_path".to_string(),
            error_message: "`path` is required".to_string(),
        });
    }

    let max_chars = clamp_tool_max_chars(max_chars);

    let resolved =
        match crate::provider::gemini::attachments::paths::resolve_attachment_path_for_local_context(
            trimmed_path,
        ) {
            Ok(path_buf) => path_buf,
            Err(message) => {
                return LocalAttachmentContextResult::Failure(LocalAttachmentContextFailure {
                    ok: false,
                    path: trimmed_path.to_string(),
                    error_code: resolution_error_code(&message).to_string(),
                    error_message: message,
                });
            }
        };

    let kind = match parse_kind(&resolved) {
        Ok(kind) => kind,
        Err(message) => {
            return LocalAttachmentContextResult::Failure(LocalAttachmentContextFailure {
                ok: false,
                path: trimmed_path.to_string(),
                error_code: "unsupported_type".to_string(),
                error_message: message,
            });
        }
    };

    let extracted = match kind {
        "text" => read_text_lossy(&resolved).await,
        _ => Err("Unsupported attachment kind".to_string()),
    };

    let extracted = match extracted {
        Ok(value) => value,
        Err(message) => {
            return LocalAttachmentContextResult::Failure(LocalAttachmentContextFailure {
                ok: false,
                path: trimmed_path.to_string(),
                error_code: "parse_error".to_string(),
                error_message: message,
            });
        }
    };

    if extracted.trim().is_empty() {
        return empty_text_error(trimmed_path, kind);
    }

    let (text, truncated) = truncate_to_char_limit(&extracted, max_chars);
    let char_count = text.chars().count();

    LocalAttachmentContextResult::Success(LocalAttachmentContextSuccess {
        ok: true,
        path: trimmed_path.to_string(),
        kind: kind.to_string(),
        text,
        truncated,
        char_count,
    })
}

fn format_success_preview_item(
    path: &str,
    success: &LocalAttachmentContextSuccess,
    template: &str,
) -> String {
    let mut vars = HashMap::new();
    vars.insert("PATH".to_string(), path.to_string());
    vars.insert("KIND".to_string(), success.kind.clone());
    vars.insert("CHAR_COUNT".to_string(), success.char_count.to_string());
    vars.insert("TRUNCATED".to_string(), success.truncated.to_string());
    vars.insert("CONTEXT".to_string(), success.text.clone());

    interpolate(template, &vars).trim().to_string()
}

fn format_error_preview_item(
    path: &str,
    failure: &LocalAttachmentContextFailure,
    template: &str,
) -> String {
    let mut vars = HashMap::new();
    vars.insert("PATH".to_string(), path.to_string());
    vars.insert("ERROR_CODE".to_string(), failure.error_code.clone());
    vars.insert("ERROR_MESSAGE".to_string(), failure.error_message.clone());

    interpolate(template, &vars).trim().to_string()
}

/// Build the `[Attachment Context]` preview block using local extraction.
///
/// Budget policy:
/// - max 6 files
/// - soft 2k chars per file
/// - hard 12k chars total
pub(crate) async fn build_attachment_preview_context(
    attachment_paths: &[String],
) -> Result<Option<String>, String> {
    if attachment_paths.is_empty() {
        return Ok(None);
    }

    let template = load_attachment_preview_context()?;
    let mut items = Vec::new();
    let mut remaining_budget = PREVIEW_HARD_MAX_TOTAL_CHARS;

    for path in attachment_paths.iter().take(PREVIEW_MAX_FILES) {
        if remaining_budget == 0 {
            break;
        }

        let per_file_budget = PREVIEW_SOFT_MAX_CHARS_PER_FILE.min(remaining_budget);
        let parsed = read_local_attachment_context(path, Some(per_file_budget)).await;

        let item = match &parsed {
            LocalAttachmentContextResult::Success(success) => {
                format_success_preview_item(path, success, &template.success_item_template)
            }
            LocalAttachmentContextResult::Failure(failure) => {
                format_error_preview_item(path, failure, &template.error_item_template)
            }
        };

        if item.trim().is_empty() {
            continue;
        }

        let (bounded_item, _) = truncate_to_char_limit(&item, remaining_budget);
        if bounded_item.trim().is_empty() {
            break;
        }

        remaining_budget = remaining_budget.saturating_sub(bounded_item.chars().count());
        items.push(bounded_item);
    }

    if items.is_empty() {
        return Ok(None);
    }

    let mut block = template.header.trim().to_string();
    block.push_str("\n\n");
    block.push_str(&items.join("\n\n"));

    let (bounded_block, _) = truncate_to_char_limit(&block, PREVIEW_HARD_MAX_TOTAL_CHARS);
    Ok(Some(bounded_block))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_tool_max_chars() {
        assert_eq!(clamp_tool_max_chars(None), 30_000);
        assert_eq!(clamp_tool_max_chars(Some(90_000)), 30_000);
        assert_eq!(clamp_tool_max_chars(Some(0)), 1);
        assert_eq!(clamp_tool_max_chars(Some(1_500)), 1_500);
    }

    #[test]
    fn truncates_to_limit() {
        let (value, truncated) = truncate_to_char_limit("abcdef", 3);
        assert_eq!(value, "abc");
        assert!(truncated);

        let (value, truncated) = truncate_to_char_limit("abc", 8);
        assert_eq!(value, "abc");
        assert!(!truncated);
    }

    #[test]
    fn text_extensions_are_supported_via_path_helper() {
        assert_eq!(parse_kind(Path::new("a.rs")).expect("kind"), "text");
        assert_eq!(parse_kind(Path::new("a.md")).expect("kind"), "text");
    }

    #[test]
    fn office_documents_are_not_supported_locally() {
        for path in [
            "a.pdf", "a.docx", "a.doc", "a.xlsx", "a.xls", "a.pptx", "a.ppt", "a.rtf", "a.odt",
            "a.ods", "a.odp",
        ] {
            let err = parse_kind(Path::new(path)).expect_err("document should be unsupported");
            assert!(err.contains("not supported by the local context reader"));
        }
    }

    #[test]
    fn unsupported_extension_returns_error() {
        let err = parse_kind(Path::new("a.bin")).expect_err("must be unsupported");
        assert!(err.contains("Unsupported file type"));
    }

    #[test]
    fn resolution_errors_map_to_codes() {
        assert_eq!(
            resolution_error_code("Attachment path is outside active chat storage scope"),
            "path_out_of_scope"
        );
        assert_eq!(
            resolution_error_code("Attachment not found: abc"),
            "attachment_not_found"
        );
        assert_eq!(resolution_error_code("random"), "path_resolution_failed");
    }
}
