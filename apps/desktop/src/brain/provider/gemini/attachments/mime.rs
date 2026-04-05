// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

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
