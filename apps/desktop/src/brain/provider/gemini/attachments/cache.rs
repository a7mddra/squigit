// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use tokio::sync::Mutex;

use super::{mime_from_extension, upload_file_to_gemini, GeminiFileRef};

fn is_uri_expired(file_ref: &GeminiFileRef) -> bool {
    if let Ok(elapsed) = file_ref.uploaded_at.elapsed() {
        elapsed.as_secs() > 47 * 3600 // 47 hours
    } else {
        true
    }
}

pub async fn ensure_file_uploaded(
    api_key: &str,
    cas_path: &str,
    cache: &Mutex<HashMap<String, GeminiFileRef>>,
) -> Result<GeminiFileRef, String> {
    let cas_hash = std::path::Path::new(cas_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    {
        let cache_lock = cache.lock().await;
        if let Some(file_ref) = cache_lock.get(&cas_hash) {
            if !is_uri_expired(file_ref) {
                return Ok(file_ref.clone());
            }
        }
    }

    let ext = std::path::Path::new(cas_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let mime_type = mime_from_extension(ext);
    let display_name = format!("{}.{}", cas_hash.chars().take(8).collect::<String>(), ext);

    let new_ref = upload_file_to_gemini(api_key, cas_path, mime_type, &display_name).await?;

    let mut cache_lock = cache.lock().await;
    cache_lock.insert(cas_hash, new_ref.clone());

    Ok(new_ref)
}
