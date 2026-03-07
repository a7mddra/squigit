// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Speech-to-text Tauri commands
//!
//! Exposes start_stt/stop_stt to frontend, streaming events via window.emit("stt_event")

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
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
fn resolve_sidecar_path(app: &AppHandle) -> Result<PathBuf, String> {
    // In dev mode: look relative to project
    // In production: look in resource dir
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let binary_name = if cfg!(windows) {
        "whisper-stt.exe"
    } else {
        "whisper-stt"
    };

    // Try resource dir first (production)
    let prod_path = resource_dir.join("binaries").join(binary_name);
    if prod_path.exists() {
        return Ok(prod_path);
    }

    // Fallback to dev path (looking for CMake build artifact)
    // resource_dir is typically target/debug
    // target/debug -> target -> root
    let root_path = resource_dir.parent().and_then(|p| p.parent());

    if let Some(root) = root_path {
        // Option A: sidecars/whisper-stt/build/whisper-stt (CMake output)
        let build_path = root.join("sidecars/whisper-stt/build").join(binary_name);
        if build_path.exists() {
            return Ok(build_path);
        }

        // Option B: app/binaries/whisper-stt (if copied without triple)
        let bin_path = root.join("app/binaries").join(binary_name);
        if bin_path.exists() {
            return Ok(bin_path);
        }

        // Option C: check for any file starting with binary_name in app/binaries (for triple)
        if let Ok(entries) = std::fs::read_dir(root.join("app/binaries")) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with(binary_name) {
                        return Ok(entry.path());
                    }
                }
            }
        }
    }

    Err(format!(
        "Sidecar not found. Searched: {:?} and dev locations",
        prod_path
    ))
}

/// Resolve model path
fn resolve_model_path(app: &AppHandle, model_name: &str) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    // Try resource dir (production)
    let prod_path = resource_dir.join("models").join(model_name);
    if prod_path.exists() {
        return Ok(prod_path);
    }

    // Try sidecar models dir (dev)
    if let Some(root) = resource_dir.parent().and_then(|p| p.parent()) {
        let dev_path = root.join("sidecars/whisper-stt/models").join(model_name);
        if dev_path.exists() {
            return Ok(dev_path);
        }
    }

    Err(format!("Model not found: {}", model_name))
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

    // Resolve paths
    let binary_path = resolve_sidecar_path(&app)?;
    let model_name = model.unwrap_or_else(|| "ggml-base.en.bin".to_string());
    let model_path = resolve_model_path(&app, &model_name)?;
    let lang = language.unwrap_or_else(|| "en".to_string());

    log::info!(
        "Starting STT: binary={:?}, model={:?}, lang={}",
        binary_path,
        model_path,
        lang
    );

    // Create and start engine
    let mut engine = SpeechEngine::new(binary_path);
    let mut rx = engine
        .start(model_path.to_string_lossy().to_string(), lang)
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
