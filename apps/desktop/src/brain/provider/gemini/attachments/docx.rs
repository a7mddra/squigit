// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use regex::Regex;
use std::io::{Cursor, Read};

fn decode_basic_xml_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn extract_text_from_docx_xml(xml: &str) -> String {
    let token_re =
        Regex::new(r#"(?s)<w:t[^>]*>(.*?)</w:t>|<w:tab\s*/>|<w:br\s*/>|<w:cr\s*/>|</w:p>|</w:tr>"#)
            .expect("DOCX token regex must compile");

    let mut out = String::new();
    for caps in token_re.captures_iter(xml) {
        if let Some(text) = caps.get(1) {
            out.push_str(&decode_basic_xml_entities(text.as_str()));
            continue;
        }

        let token = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
        if token.starts_with("<w:tab") {
            out.push('\t');
        } else {
            out.push('\n');
        }
    }

    // Light normalization to avoid very noisy spacing.
    let mut normalized = String::new();
    let mut last_was_newline = false;
    for ch in out.chars() {
        if ch == '\n' {
            if !last_was_newline {
                normalized.push('\n');
            }
            last_was_newline = true;
        } else {
            normalized.push(ch);
            last_was_newline = false;
        }
    }

    normalized.trim().to_string()
}

fn extract_docx_text_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let reader = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid DOCX zip: {}", e))?;

    let mut document_xml = String::new();
    let mut doc_entry = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("Missing DOCX document.xml: {}", e))?;
    doc_entry
        .read_to_string(&mut document_xml)
        .map_err(|e| format!("Failed reading DOCX document.xml: {}", e))?;

    let text = extract_text_from_docx_xml(&document_xml);
    if text.is_empty() {
        return Err("DOCX has no readable text".to_string());
    }

    Ok(text)
}

pub async fn extract_docx_text_for_prompt(file_path: &str) -> Result<String, String> {
    const MAX_DOCX_CHARS: usize = 120_000;

    let bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| format!("Failed to read DOCX file: {}", e))?;

    let text = tokio::task::spawn_blocking(move || extract_docx_text_from_bytes(&bytes))
        .await
        .map_err(|e| format!("DOCX extraction task failed: {}", e))??;

    if text.chars().count() > MAX_DOCX_CHARS {
        let truncated: String = text.chars().take(MAX_DOCX_CHARS).collect();
        return Ok(format!(
            "{}\n\n[Truncated: document exceeded {} characters.]",
            truncated, MAX_DOCX_CHARS
        ));
    }

    Ok(text)
}
