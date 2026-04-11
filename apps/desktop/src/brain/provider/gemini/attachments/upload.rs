// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use chrono::{Duration, Utc};
use reqwest::{header, Client};

use super::types::{GeminiFileObject, GeminiFileUploadFinalizeResponse};
use super::GeminiFileRef;

pub async fn upload_file_to_gemini(
    api_key: &str,
    file_path: &str,
    mime_type: &str,
    display_name: &str,
) -> Result<GeminiFileRef, String> {
    let client = Client::new();
    let file_bytes = tokio::fs::read(file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let file_size = file_bytes.len();

    // Step 1: Start Resumable Upload
    let start_url = format!(
        "https://generativelanguage.googleapis.com/upload/v1beta/files?key={}",
        api_key
    );

    let mut headers = header::HeaderMap::new();
    headers.insert(
        "X-Goog-Upload-Protocol",
        header::HeaderValue::from_static("resumable"),
    );
    headers.insert(
        "X-Goog-Upload-Command",
        header::HeaderValue::from_static("start"),
    );
    headers.insert(
        "X-Goog-Upload-Header-Content-Length",
        file_size.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-Goog-Upload-Header-Content-Type",
        header::HeaderValue::from_str(mime_type).unwrap(),
    );

    let body = serde_json::json!({
        "file": {
            "display_name": display_name
        }
    });

    let res1 = client
        .post(&start_url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Start upload failed: {}", e))?;

    if !res1.status().is_success() {
        let text = res1.text().await.unwrap_or_default();
        return Err(format!("Gemini API Error (Upload Start): {}", text));
    }

    let upload_url = res1
        .headers()
        .get("X-Goog-Upload-URL")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing X-Goog-Upload-URL header")?
        .to_string();

    // Step 2: Upload Bytes
    let mut headers2 = header::HeaderMap::new();
    headers2.insert(
        "X-Goog-Upload-Offset",
        header::HeaderValue::from_static("0"),
    );
    headers2.insert(
        "X-Goog-Upload-Command",
        header::HeaderValue::from_static("upload, finalize"),
    );
    headers2.insert(
        header::CONTENT_LENGTH,
        file_size.to_string().parse().unwrap(),
    );

    let res2 = client
        .put(&upload_url)
        .headers(headers2)
        .body(file_bytes)
        .send()
        .await
        .map_err(|e| format!("Finalize upload failed: {}", e))?;

    if !res2.status().is_success() {
        let text = res2.text().await.unwrap_or_default();
        return Err(format!("Gemini API Error (Upload Finalize): {}", text));
    }

    let final_res: GeminiFileUploadFinalizeResponse = res2
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))?;

    // Step 3: Poll for ACTIVE state if needed
    let file_obj = final_res.file;
    let name = file_obj.name.ok_or("Missing file name")?;
    let uri = file_obj.uri.ok_or("Missing file uri")?;

    poll_file_status(api_key, &name).await?;

    Ok(GeminiFileRef {
        file_uri: uri,
        file_name: name,
        mime_type: mime_type.to_string(),
        display_name: display_name.to_string(),
        uploaded_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(47),
    })
}

pub async fn poll_file_status(api_key: &str, file_name: &str) -> Result<(), String> {
    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/{}?key={}",
        file_name, api_key
    );

    loop {
        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Poll failed: {}", e))?;
        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(format!("Gemini API Error (Poll): {}", text));
        }

        let file_obj: GeminiFileObject =
            res.json().await.map_err(|e| format!("Poll parse: {}", e))?;
        if let Some(state) = file_obj.state {
            if state == "ACTIVE" {
                break;
            } else if state == "FAILED" {
                return Err("File processing failed".to_string());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }
    Ok(())
}
