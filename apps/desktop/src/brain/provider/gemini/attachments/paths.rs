// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_chat_storage::ChatStorage;
use ops_profile_store::ProfileStore;

pub(crate) fn get_active_storage() -> Result<ChatStorage, String> {
    let profile_store = ProfileStore::new().map_err(|e| e.to_string())?;
    let active_id = profile_store
        .get_active_profile_id()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active profile. Please log in first.".to_string())?;

    let chats_dir = profile_store.get_chats_dir(&active_id);
    ChatStorage::with_base_dir(chats_dir).map_err(|e| e.to_string())
}

fn is_path_within_base(path: &std::path::Path, base: &std::path::Path) -> bool {
    path.starts_with(base)
}

pub(crate) fn resolve_attachment_path_internal(path: &str) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::path::PathBuf;

    let incoming = PathBuf::from(path);
    if incoming.is_absolute() {
        if incoming.exists() {
            return fs::canonicalize(&incoming).map_err(|e| e.to_string());
        }
        return Err(format!("Attachment not found: {}", path));
    }

    let storage = get_active_storage()?;

    let from_base_dir = storage.base_dir().join(&incoming);
    if from_base_dir.exists() {
        return fs::canonicalize(&from_base_dir).map_err(|e| e.to_string());
    }

    // Legacy fallback: resolve objects/<prefix>/<hash>.<ext> by hash, regardless of extension.
    if let Some(file_name) = incoming.file_name().and_then(|name| name.to_str()) {
        if let Some((hash, _ext)) = file_name.split_once('.') {
            if hash.len() >= 2 {
                let prefix = &hash[..2];
                let prefix_dir = storage.objects_dir().join(prefix);

                if let Ok(entries) = fs::read_dir(prefix_dir) {
                    for entry in entries.flatten() {
                        let candidate = entry.path();
                        let stem = candidate.file_stem().and_then(|s| s.to_str());
                        if stem == Some(hash) {
                            return fs::canonicalize(candidate).map_err(|e| e.to_string());
                        }
                    }
                }
            }
        }
    }

    Err(format!("Attachment not found: {}", path))
}

/// Resolve and enforce the attachment path to be inside the active profile chat storage scope.
pub(crate) fn resolve_attachment_path_for_local_context(
    path: &str,
) -> Result<std::path::PathBuf, String> {
    let resolved = resolve_attachment_path_internal(path)?;
    let storage = get_active_storage()?;
    let base_dir = std::fs::canonicalize(storage.base_dir()).map_err(|e| {
        format!(
            "Failed to canonicalize active chat storage directory: {}",
            e
        )
    })?;

    if is_path_within_base(&resolved, &base_dir) {
        return Ok(resolved);
    }

    Err(format!(
        "Attachment path is outside active chat storage scope: {}",
        path
    ))
}

#[cfg(test)]
mod tests {
    use super::is_path_within_base;
    use std::path::Path;

    #[test]
    fn scope_check_accepts_child_paths() {
        let base = Path::new("/tmp/chats");
        let child = Path::new("/tmp/chats/objects/ab/file.txt");
        assert!(is_path_within_base(child, base));
    }

    #[test]
    fn scope_check_rejects_siblings() {
        let base = Path::new("/tmp/chats");
        let other = Path::new("/tmp/other/file.txt");
        assert!(!is_path_within_base(other, base));
    }
}
