// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

mod cache;
mod docx;
mod mime;
mod types;
mod upload;

pub use cache::ensure_file_uploaded;
pub use docx::extract_docx_text_for_prompt;
pub use mime::{is_docx_path, mime_from_extension};
pub use types::GeminiFileRef;
pub use upload::{poll_file_status, upload_file_to_gemini};
