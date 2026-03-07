use regex::Regex;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::time::SystemTime;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFileRef {
    pub file_uri: String,
    pub mime_type: String,
    pub display_name: String,
    pub uploaded_at: SystemTime,
}

#[derive(Debug, Deserialize)]
struct GeminiFileObject {
    name: Option<String>,
    uri: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiFileUploadFinalizeResponse {
    file: GeminiFileObject,
}

pub fn mime_from_extension(ext: &str) -> &str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "rtf" => "application/rtf",
        "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "css" | "html" | "md" | "txt" | "csv"
        | "json" | "xml" | "yml" | "yaml" | "toml" | "sh" | "bash" | "c" | "cpp" | "h" | "hpp"
        | "java" | "go" | "php" | "rb" | "swift" | "kt" | "sql" => "text/plain",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

pub fn is_docx_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("docx"))
        .unwrap_or(false)
}

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
        mime_type: mime_type.to_string(),
        display_name: display_name.to_string(),
        uploaded_at: SystemTime::now(),
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
