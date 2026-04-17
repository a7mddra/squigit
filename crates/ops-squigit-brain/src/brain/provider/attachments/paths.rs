// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
pub(crate) fn resolve_attachment_path_internal(path: &str) -> Result<std::path::PathBuf, String> {
    crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal(path)
}
