// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use tokio::sync::Mutex;

use super::{mime_from_extension, upload_file_to_gemini, GeminiFileRef};

fn is_uri_expired(file_ref: &GeminiFileRef) -> bool {
    chrono::Utc::now() >= file_ref.expires_at
}

pub async fn ensure_file_uploaded(
    api_key: &str,
    cas_path: &str,
    cache: &Mutex<HashMap<String, GeminiFileRef>>,
) -> Result<GeminiFileRef, String> {
    let resolved_path =
        crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal(
            cas_path,
        )?;

    let cas_hash = resolved_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let cache_key = format!("{}_{}", cas_hash, &api_key[api_key.len().saturating_sub(6)..]);

    {
        let cache_lock = cache.lock().await;
        if let Some(file_ref) = cache_lock.get(&cache_key) {
            if !is_uri_expired(file_ref) {
                return Ok(file_ref.clone());
            }
        }
    }

    let ext = resolved_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let mime_type = mime_from_extension(ext);
    let display_name = format!("{}.{}", cas_hash.chars().take(8).collect::<String>(), ext);

    let resolved_str = resolved_path.to_string_lossy().to_string();
    let new_ref = upload_file_to_gemini(api_key, &resolved_str, mime_type, &display_name).await?;

    let mut cache_lock = cache.lock().await;
    cache_lock.insert(cache_key, new_ref.clone());

    Ok(new_ref)
}
