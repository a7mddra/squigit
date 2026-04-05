// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct GeminiRequestControl {
    pub(crate) cancel_token: tokio_util::sync::CancellationToken,
    answer_now: Arc<AtomicBool>,
    pub(crate) answer_now_notify: Arc<tokio::sync::Notify>,
}

impl GeminiRequestControl {
    pub(crate) fn new() -> Self {
        Self {
            cancel_token: tokio_util::sync::CancellationToken::new(),
            answer_now: Arc::new(AtomicBool::new(false)),
            answer_now_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    fn request_answer_now(&self) {
        self.answer_now.store(true, Ordering::SeqCst);
        self.answer_now_notify.notify_waiters();
    }

    pub(crate) fn is_answer_now_requested(&self) -> bool {
        self.answer_now.load(Ordering::SeqCst)
    }
}

lazy_static::lazy_static! {
    static ref ACTIVE_REQUESTS: std::sync::Arc<tokio::sync::Mutex<HashMap<String, GeminiRequestControl>>> = std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new()));
}

pub(crate) async fn register_request(channel_id: String, control: GeminiRequestControl) {
    let mut map = ACTIVE_REQUESTS.lock().await;
    map.insert(channel_id, control);
}

pub(crate) async fn remove_request(channel_id: &str) {
    let mut map = ACTIVE_REQUESTS.lock().await;
    map.remove(channel_id);
}

#[tauri::command]
pub async fn cancel_gemini_request(channel_id: Option<String>) -> Result<(), String> {
    let mut map = ACTIVE_REQUESTS.lock().await;
    if let Some(id) = channel_id {
        log::info!("Cancelling request for channel: {}", id);
        if let Some(control) = map.remove(&id) {
            control.cancel_token.cancel();
        }
    } else {
        log::info!("Cancelling ALL Gemini requests");
        for (_, control) in map.drain() {
            control.cancel_token.cancel();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn answer_now_gemini_request(channel_id: String) -> Result<(), String> {
    let map = ACTIVE_REQUESTS.lock().await;
    if let Some(control) = map.get(&channel_id) {
        log::info!("Answer-now requested for channel: {}", channel_id);
        control.request_answer_now();
    } else {
        log::info!(
            "Answer-now requested for unknown channel (likely completed): {}",
            channel_id
        );
    }
    Ok(())
}
