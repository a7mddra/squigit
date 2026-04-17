// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::provider::gemini::transport::types::GeminiEvent;

pub trait BrainEventSink: Send + Sync {
    fn emit(&self, channel_id: &str, event: GeminiEvent);
}

pub struct NoopEventSink;

impl BrainEventSink for NoopEventSink {
    fn emit(&self, _channel_id: &str, _event: GeminiEvent) {}
}
