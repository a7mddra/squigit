// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

pub mod attachment_paths;
pub mod files;

mod attachments;
mod fallback;
pub(crate) mod generation;
pub(crate) mod request_control;
mod search_helpers;
pub(crate) mod stream_chat;
mod streaming;
mod types;

pub use generation::{compress_conversation, generate_chat_title, generate_image_brief};
pub use request_control::{answer_now_gemini_request, cancel_gemini_request};
pub use stream_chat::stream_gemini_chat_v2;
