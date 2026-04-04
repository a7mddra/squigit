// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

use super::constants::MAX_RETRIES;
use super::types::SearchError;

pub(crate) fn emit_progress(
    progress: &mut Option<&mut (dyn FnMut(String) + Send)>,
    message: impl Into<String>,
) {
    if let Some(cb) = progress.as_deref_mut() {
        cb(message.into());
    }
}

pub(crate) async fn with_retries_with_progress<T, F, Fut>(
    _label: &str,
    mut op: F,
    _progress: &mut Option<&mut (dyn FnMut(String) + Send)>,
) -> Result<T, SearchError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, SearchError>>,
{
    let mut attempt = 0usize;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if !e.retriable || attempt >= MAX_RETRIES {
                    return Err(e);
                }
                attempt += 1;
                let exp = 2u64.pow(attempt as u32);
                let jitter = rand::thread_rng().gen_range(80..220);
                let wait_ms = exp * 220 + jitter;
                sleep(Duration::from_millis(wait_ms)).await;
            }
        }
    }
}
