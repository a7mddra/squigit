// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use futures_util::StreamExt;
use crate::events::BrainEventSink;

use super::types::{GeminiEvent, GeminiFunctionCall, GeminiRequest, GeminiResponseChunk};

pub(crate) fn emit_event(sink: &dyn BrainEventSink, channel_id: &str, event: GeminiEvent) {
    sink.emit(channel_id, event);
}

pub(crate) struct StreamIterationResult {
    pub(crate) function_call: Option<GeminiFunctionCall>,
    pub(crate) function_call_thought_signature: Option<String>,
}

pub(crate) async fn stream_request_iteration(
    sink: &dyn BrainEventSink,
    client: &reqwest::Client,
    url: &str,
    request_body: &GeminiRequest,
    channel_id: &str,
    cancel_token: &tokio_util::sync::CancellationToken,
) -> Result<StreamIterationResult, String> {
    let response_result = tokio::select! {
        res = client.post(url).json(request_body).send() => res.map_err(|e| format!("Failed to send request to Gemini: {}", e)),
        _ = cancel_token.cancelled() => Err("CANCELLED".to_string()),
    };
    let response = response_result?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Gemini API Error: {}", error_text));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();
    let mut function_call: Option<GeminiFunctionCall> = None;
    let mut function_call_thought_signature: Option<String> = None;

    'stream_loop: loop {
        tokio::select! {
            chunk_opt = stream.next() => {
                match chunk_opt {
                    Some(Ok(chunk)) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(idx) = buffer.find('\n') {
                            let line = buffer[..idx].to_string();
                            buffer.drain(..idx + 1);

                            let trimmed = line.trim();
                            if let Some(data) = trimmed.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    break 'stream_loop;
                                }
                                let Ok(chunk_data) = serde_json::from_str::<GeminiResponseChunk>(data) else {
                                    continue;
                                };
                                let Some(candidates) = chunk_data.candidates else {
                                    continue;
                                };
                                let Some(first) = candidates.first() else {
                                    continue;
                                };

                                if let Some(content) = &first.content {
                                    if let Some(parts) = &content.parts {
                                        for part in parts {
                                            if let Some(fc) = &part.function_call {
                                                if function_call.is_none() {
                                                    function_call = Some(fc.clone());
                                                    function_call_thought_signature =
                                                        part.thought_signature.clone();
                                                    if !full_text.is_empty() {
                                                        full_text.clear();
                                                        emit_event(sink, channel_id, GeminiEvent::Reset);
                                                    }
                                                }
                                                continue;
                                            }
                                            if function_call.is_none() {
                                                if let Some(text) = &part.text {
                                                    full_text.push_str(text);
                                                    emit_event(
                                                        sink,
                                                        channel_id,
                                                        GeminiEvent::Token {
                                                            token: text.clone(),
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }

                                if first.finish_reason.is_some() {
                                    break 'stream_loop;
                                }
                            }
                        }
                    }
                    Some(Err(e)) => return Err(format!("Stream error: {}", e)),
                    None => break,
                }
            }
            _ = cancel_token.cancelled() => return Err("CANCELLED".to_string()),
        }
    }

    Ok(StreamIterationResult {
        function_call,
        function_call_thought_signature,
    })
}
