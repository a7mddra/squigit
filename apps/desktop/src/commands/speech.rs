// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Speech-to-text Tauri commands
//!
//! Exposes start_stt/stop_stt to frontend, streaming events via window.emit("stt_event")

use std::path::{Path, PathBuf};
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
fn resolve_sidecar_path(app: &AppHandle) -> Result<(PathBuf, Option<PathBuf>), String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let runtime_dir_name = get_runtime_dir_name();
    let binary_name = get_sidecar_executable_name();

    let runtime_candidates = [
        resource_dir.join("binaries").join(&runtime_dir_name),
        resource_dir.join(&runtime_dir_name),
        resource_dir
            .join("resources")
            .join("binaries")
            .join(&runtime_dir_name),
    ];
    for runtime_dir in runtime_candidates {
        let sidecar = runtime_dir.join(binary_name);
        if sidecar.exists() {
            return Ok((sidecar, Some(runtime_dir)));
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(root) = exe_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let dev_candidates = [
                root.join("apps")
                    .join("desktop")
                    .join("binaries")
                    .join(&runtime_dir_name)
                    .join(binary_name),
                root.join("target")
                    .join("debug")
                    .join("binaries")
                    .join(&runtime_dir_name)
                    .join(binary_name),
                root.join("sidecars")
                    .join("whisper-stt")
                    .join("build")
                    .join("Release")
                    .join(binary_name),
                root.join("sidecars")
                    .join("whisper-stt")
                    .join("build")
                    .join(binary_name),
            ];

            for sidecar in dev_candidates {
                if sidecar.exists() {
                    let runtime_dir = infer_runtime_dir(&sidecar);
                    return Ok((sidecar, runtime_dir));
                }
            }
        }
    }

    Err(format!(
        "Whisper sidecar not found. Expected runtime layout at binaries/{}/{}",
        runtime_dir_name, binary_name
    ))
}

fn infer_runtime_dir(binary_path: &Path) -> Option<PathBuf> {
    let parent = binary_path.parent()?;
    if parent.join("_internal").is_dir() {
        Some(parent.to_path_buf())
    } else {
        None
    }
}

fn get_target_triple() -> &'static str {
    const FALLBACK_TARGET: &str = {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            "x86_64-pc-windows-msvc"
        }
        #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
        {
            "aarch64-pc-windows-msvc"
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            "x86_64-unknown-linux-gnu"
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            "aarch64-unknown-linux-gnu"
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            "x86_64-apple-darwin"
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            "aarch64-apple-darwin"
        }
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "aarch64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64")
        )))]
        {
            "unknown-target"
        }
    };

    option_env!("TARGET").unwrap_or(FALLBACK_TARGET)
}

fn get_runtime_dir_name() -> String {
    format!("whisper-stt-{}", get_target_triple())
}

fn get_sidecar_executable_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "whisper-stt.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "whisper-stt"
    }
}

/// Resolve model path
fn resolve_model_path(
    app: &AppHandle,
    runtime_dir: Option<&Path>,
    model_name: &str,
) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    let mut searched = Vec::new();

    if let Some(runtime) = runtime_dir {
        let prod = runtime.join("_internal").join("models").join(model_name);
        searched.push(prod.display().to_string());
        if prod.exists() {
            return Ok(prod);
        }
    }

    let runtime_dir_name = get_runtime_dir_name();
    let prod_candidates = [
        resource_dir
            .join("binaries")
            .join(&runtime_dir_name)
            .join("_internal")
            .join("models")
            .join(model_name),
        resource_dir
            .join(&runtime_dir_name)
            .join("_internal")
            .join("models")
            .join(model_name),
        resource_dir
            .join("resources")
            .join("binaries")
            .join(&runtime_dir_name)
            .join("_internal")
            .join("models")
            .join(model_name),
    ];

    for candidate in prod_candidates {
        searched.push(candidate.display().to_string());
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(root) = exe_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let dev_path = root
                .join("sidecars")
                .join("whisper-stt")
                .join("models")
                .join(model_name);
            searched.push(dev_path.display().to_string());
            if dev_path.exists() {
                return Ok(dev_path);
            }
        }
    }

    Err(format!(
        "Whisper model not found: {}. Searched:\n  - {}",
        model_name,
        searched.join("\n  - ")
    ))
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
    let (binary_path, runtime_dir) = resolve_sidecar_path(&app)?;
    let model_name = model.unwrap_or_else(|| "ggml-tiny.en.bin".to_string());
    let model_path = resolve_model_path(&app, runtime_dir.as_deref(), &model_name)?;
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
