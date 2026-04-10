// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

const TEXT_LIKE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "css", "html", "md", "txt", "csv", "json", "xml", "yml",
    "yaml", "toml", "sh", "bash", "c", "cpp", "h", "hpp", "java", "go", "php", "rb", "swift", "kt",
    "sql", "rst", "ini", "cfg", "conf", "env", "log",
];

const GEMINI_DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "docm", "xls", "xlsx", "xlsm", "ppt", "pptx", "pptm", "rtf", "odt",
    "ods", "odp",
];

fn normalized_extension(value: &str) -> String {
    std::path::Path::new(value)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase()
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
        "xlsm" => "application/vnd.ms-excel.sheet.macroenabled.12",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "pptm" => "application/vnd.ms-powerpoint.presentation.macroenabled.12",
        "rtf" => "application/rtf",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "css" | "html" | "md" | "txt" | "csv"
        | "json" | "xml" | "yml" | "yaml" | "toml" | "sh" | "bash" | "c" | "cpp" | "h" | "hpp"
        | "java" | "go" | "php" | "rb" | "swift" | "kt" | "sql" | "rst" | "ini" | "cfg"
        | "conf" | "env" | "log" => "text/plain",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

pub fn is_text_like_path(path: &str) -> bool {
    let extension = normalized_extension(path);
    TEXT_LIKE_EXTENSIONS.contains(&extension.as_str())
}

pub fn is_gemini_document_path(path: &str) -> bool {
    let extension = normalized_extension(path);
    GEMINI_DOCUMENT_EXTENSIONS.contains(&extension.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_like_paths_are_detected() {
        assert!(is_text_like_path("objects/ab/file.rs"));
        assert!(is_text_like_path("/tmp/file.txt"));
        assert!(!is_text_like_path("objects/ab/file.docx"));
    }

    #[test]
    fn gemini_document_paths_are_detected() {
        for path in [
            "report.pdf",
            "lecture.docx",
            "sheet.xlsx",
            "slides.pptx",
            "notes.odt",
            "table.ods",
            "deck.odp",
        ] {
            assert!(
                is_gemini_document_path(path),
                "expected document path: {}",
                path
            );
        }
        assert!(!is_gemini_document_path("main.rs"));
    }
}
