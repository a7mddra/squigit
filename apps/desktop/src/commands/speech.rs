// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Speech-to-text Tauri commands
//!
//! Exposes start_stt/stop_stt to frontend, streaming events via window.emit("stt_event")

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use svc_speech_engine::{SpeechEngine, SttEvent};

/// Shared speech engine state
pub struct SpeechState {
    pub engine: Arc<Mutex<Option<SpeechEngine>>>,
}

impl Default for SpeechState {
    fn default() -> Self {
        Self {
            engine: Arc::new(Mutex::new(None)),
        }
    }
}

/// Resolve the whisper-stt sidecar binary path
fn resolve_sidecar_path(_app: &AppHandle) -> Result<(PathBuf, Option<PathBuf>), String> {
    let system_cmd = if cfg!(target_os = "windows") { "squigit-stt.exe" } else { "squigit-stt" };
    if which_cmd(system_cmd) {
        return Ok((PathBuf::from(system_cmd), None));
    }

    Err("ERR_MISSING_STT_PACKAGE".to_string())
}

fn which_cmd(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in std::env::split_paths(&path) {
            let exe = p.join(cmd);
            if exe.exists() {
                return true;
            }
        }
    }
    false
}





#[tauri::command]
pub async fn start_stt(
    app: AppHandle,
    state: State<'_, SpeechState>,
    model: Option<String>,
    language: Option<String>,
) -> Result<(), String> {
    let mut engine_guard = state.engine.lock().await;

    if engine_guard.is_some() {
        return Err("STT already running".to_string());
    }

    let (binary_path, _) = resolve_sidecar_path(&app)?;
    let model_name = model.unwrap_or_else(|| "ggml-tiny.en.bin".to_string());
    let lang = language.unwrap_or_else(|| "en".to_string());

    log::info!(
        "Starting STT: binary={:?}, model={:?}, lang={}",
        binary_path,
        model_name,
        lang
    );

    // Create and start engine
    let mut engine = SpeechEngine::new(binary_path);
    let mut rx = engine
        .start(model_name, lang)
        .await
        .map_err(|e| format!("Failed to start engine: {}", e))?;

    *engine_guard = Some(engine);

    // Spawn event forwarding task
    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let payload = match &event {
                SttEvent::Transcription { text, is_final } => {
                    serde_json::json!({
                        "type": "transcription",
                        "text": text,
                        "is_final": is_final
                    })
                }
                SttEvent::Status { status } => {
                    serde_json::json!({
                        "type": "status",
                        "status": status
                    })
                }
                SttEvent::Error { message } => {
                    serde_json::json!({
                        "type": "error",
                        "message": message
                    })
                }
            };

            if let Err(e) = app_handle.emit("stt_event", payload) {
                log::error!("Failed to emit stt_event: {}", e);
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_stt(state: State<'_, SpeechState>) -> Result<(), String> {
    let mut engine_guard = state.engine.lock().await;

    if let Some(mut engine) = engine_guard.take() {
        engine
            .stop()
            .await
            .map_err(|e| format!("Failed to stop engine: {}", e))?;
    }

    Ok(())
}
