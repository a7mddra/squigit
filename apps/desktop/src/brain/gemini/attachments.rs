// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::future::join_all;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

use super::types::{GeminiFileData, GeminiPart};

pub(crate) async fn build_interleaved_parts(
    text: &str,
    api_key: &str,
    cache: &Arc<tokio::sync::Mutex<HashMap<String, crate::brain::gemini::files::GeminiFileRef>>>,
) -> Result<Vec<GeminiPart>, String> {
    enum PreparedAttachment {
        Uploaded(crate::brain::gemini::files::GeminiFileRef),
        InlineText(String),
    }

    let re = Regex::new(r"\{\{([^}]+)\}\}").map_err(|e| format!("Regex Error: {}", e))?;

    let mut text_chunks = Vec::new();
    let mut last_end = 0;
    let mut file_paths = Vec::new();

    for cap in re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let path = cap.get(1).unwrap().as_str().to_string();

        let before = &text[last_end..full_match.start()];
        if !before.trim().is_empty() {
            text_chunks.push((false, before.to_string()));
        }

        file_paths.push(path.clone());
        text_chunks.push((true, path));

        last_end = full_match.end();
    }

    let remaining = &text[last_end..];
    if !remaining.trim().is_empty() {
        text_chunks.push((false, remaining.to_string()));
    }

    if file_paths.is_empty() {
        return Ok(vec![GeminiPart {
            text: Some(text.to_string()),
            ..Default::default()
        }]);
    }

    let mut unique_paths = file_paths.clone();
    unique_paths.sort();
    unique_paths.dedup();

    let prepare_futures = unique_paths.iter().map(|p| async {
        if crate::brain::gemini::files::is_docx_path(p) {
            let extracted_text =
                crate::brain::gemini::files::extract_docx_text_for_prompt(p).await?;
            Ok::<PreparedAttachment, String>(PreparedAttachment::InlineText(extracted_text))
        } else {
            let file_ref =
                crate::brain::gemini::files::ensure_file_uploaded(api_key, p, cache).await?;
            Ok::<PreparedAttachment, String>(PreparedAttachment::Uploaded(file_ref))
        }
    });

    let results = join_all(prepare_futures).await;
    let mut prepared_attachments = HashMap::new();

    for (path, result) in unique_paths.into_iter().zip(results.into_iter()) {
        match result {
            Ok(prepared) => {
                prepared_attachments.insert(path, prepared);
            }
            Err(e) => return Err(e),
        }
    }

    let mut parts = Vec::new();
    for (is_file, content) in text_chunks {
        if is_file {
            if let Some(prepared) = prepared_attachments.get(&content) {
                match prepared {
                    PreparedAttachment::Uploaded(file_ref) => {
                        parts.push(GeminiPart {
                            file_data: Some(GeminiFileData {
                                mime_type: file_ref.mime_type.clone(),
                                file_uri: file_ref.file_uri.clone(),
                            }),
                            ..Default::default()
                        });
                    }
                    PreparedAttachment::InlineText(extracted_text) => {
                        let file_name = std::path::Path::new(&content)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("attachment.docx");
                        let docx_block = format!(
                            "[Attachment: {} | format: docx | content: extracted text]\n{}\n[End attachment]",
                            file_name, extracted_text
                        );
                        parts.push(GeminiPart {
                            text: Some(docx_block),
                            ..Default::default()
                        });
                    }
                }
            }
        } else {
            parts.push(GeminiPart {
                text: Some(content),
                ..Default::default()
            });
        }
    }

    Ok(parts)
}
