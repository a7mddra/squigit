// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

pub fn resolve_attachment_path_buf(path: &str) -> Result<std::path::PathBuf, String> {
    crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal(path)
}

pub fn resolve_attachment_path(path: &str) -> Result<String, String> {
    let resolved = resolve_attachment_path_buf(path)?;
    Ok(resolved.to_string_lossy().to_string())
}

pub fn validate_text_file(path: &str) -> Result<bool, String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 8192];
    let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
    buffer.truncate(bytes_read);

    if bytes_read == 0 {
        return Ok(true);
    }

    if buffer.contains(&0) {
        return Ok(false);
    }

    match std::str::from_utf8(&buffer) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

pub fn read_attachment_text(path: &str) -> Result<String, String> {
    let resolved = resolve_attachment_path_buf(path)?;
    let bytes = std::fs::read(resolved).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}
