// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! OCR command module for Tauri <-> Python sidecar execution.
//!
//! This module runs OCR by spawning the PaddleOCR sidecar in CLI mode
//! with an image path argument and parsing JSON from stdout.
//!
//! Safety controls:
//! - Thread-limiting env vars (defense-in-depth, Python also sets them)
//! - Lower process priority via nice(10) on Unix
//! - I/O priority via ionice on Linux
//! - Timeout with automatic process kill (120s default)
//! - Single-job mutex to prevent concurrent OCR calls
//! - Cancellation via cross-platform process termination

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::LazyLock;
use tauri::Manager;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[cfg(unix)]
use std::io;

use crate::commands::chat::resolve_attachment_path_internal;
use crate::services::models::ModelManager;
use crate::state::{AppState, OcrJobHandle};

/// Maximum wall-clock time for a single OCR job (seconds).
const OCR_TIMEOUT_SECS_DEFAULT: u64 = 120;

/// Global mutex to ensure only one OCR job runs at a time.
/// Prevents concurrent calls from compounding CPU pressure.
static OCR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn get_ocr_timeout_secs() -> u64 {
    std::env::var("SNAPLLM_OCR_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(OCR_TIMEOUT_SECS_DEFAULT)
}

fn prepend_env_path(cmd: &mut tokio::process::Command, key: &str, path: &Path, sep: &str) {
    let prepend_value = path.to_string_lossy().to_string();
    let current = std::env::var(key).ok();

    let merged = match current {
        Some(existing) if !existing.is_empty() => {
            let already_present = existing.split(sep).any(|entry| entry == prepend_value);
            if already_present {
                existing
            } else {
                format!("{}{}{}", prepend_value, sep, existing)
            }
        }
        _ => prepend_value,
    };

    cmd.env(key, merged);
}

fn apply_runtime_lib_env(cmd: &mut tokio::process::Command, runtime_dir: Option<&Path>) {
    let Some(runtime_dir) = runtime_dir else {
        return;
    };

    if !runtime_dir.is_dir() {
        return;
    }

    #[cfg(windows)]
    {
        prepend_env_path(cmd, "PATH", runtime_dir, ";");
        let paddle_lib_dir = runtime_dir.join("paddle").join("libs");
        if paddle_lib_dir.is_dir() {
            prepend_env_path(cmd, "PATH", &paddle_lib_dir, ";");
        }
    }

    #[cfg(target_os = "linux")]
    {
        prepend_env_path(cmd, "PATH", runtime_dir, ":");
        let paddle_lib_dir = runtime_dir.join("paddle").join("libs");
        if paddle_lib_dir.is_dir() {
            prepend_env_path(cmd, "LD_LIBRARY_PATH", &paddle_lib_dir, ":");
            prepend_env_path(cmd, "PATH", &paddle_lib_dir, ":");
        }
    }

    #[cfg(target_os = "macos")]
    {
        prepend_env_path(cmd, "PATH", runtime_dir, ":");
        let paddle_lib_dir = runtime_dir.join("paddle").join("libs");
        if paddle_lib_dir.is_dir() {
            prepend_env_path(cmd, "DYLD_LIBRARY_PATH", &paddle_lib_dir, ":");
            prepend_env_path(cmd, "PATH", &paddle_lib_dir, ":");
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBox {
    pub text: String,
    pub box_coords: Vec<Vec<f64>>,
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Deserialize)]
struct RawOcrResult {
    text: String,
    #[serde(rename = "box")]
    bounding_box: Vec<Vec<f64>>,
    #[serde(default)]
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct OcrError {
    error: String,
}

/// Kill sidecar process and wait for shutdown.
async fn cancel_job_handle(mut handle: OcrJobHandle) {
    #[cfg(unix)]
    {
        fn signal_process_group(pid: u32, sig: i32) {
            let group_id = -(pid as i32);
            unsafe {
                libc::kill(group_id, sig);
            }
        }

        if let Some(pid) = handle.child.id() {
            signal_process_group(pid, libc::SIGINT);
        }
        if timeout(Duration::from_millis(300), handle.child.wait())
            .await
            .is_ok()
        {
            return;
        }

        if let Some(pid) = handle.child.id() {
            signal_process_group(pid, libc::SIGTERM);
        }
        if timeout(Duration::from_millis(1200), handle.child.wait())
            .await
            .is_ok()
        {
            return;
        }

        if let Some(pid) = handle.child.id() {
            signal_process_group(pid, libc::SIGHUP);
        }
        let _ = timeout(Duration::from_millis(800), handle.child.wait()).await;
        return;
    }

    #[cfg(windows)]
    {
        let _ = handle.child.start_kill();
        let _ = timeout(Duration::from_millis(800), handle.child.wait()).await;
        return;
    }
}

async fn read_pipe_to_string(pipe: Option<tokio::process::ChildStdout>) -> String {
    if let Some(mut pipe) = pipe {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    }
}

async fn read_stderr_to_string(pipe: Option<tokio::process::ChildStderr>) -> String {
    if let Some(mut pipe) = pipe {
        let mut buf = Vec::new();
        let _ = pipe.read_to_end(&mut buf).await;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    }
}

fn extract_json_payload(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if serde_json::from_str::<Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }

    for line in trimmed.lines().rev() {
        let candidate = line.trim();
        if candidate.is_empty() {
            continue;
        }
        if !candidate.starts_with('{') && !candidate.starts_with('[') {
            continue;
        }
        if serde_json::from_str::<Value>(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }

    let candidates: Vec<usize> = trimmed
        .char_indices()
        .filter_map(|(idx, ch)| {
            if ch == '{' || ch == '[' {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    for idx in candidates.into_iter().rev() {
        let candidate = &trimmed[idx..];
        if serde_json::from_str::<Value>(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }

    None
}

#[tauri::command]
pub async fn ocr_image(
    app: tauri::AppHandle,
    image_data: String,
    is_base64: bool,
    model_name: Option<String>,
) -> Result<Vec<OcrBox>, String> {
    let _guard = OCR_LOCK.lock().await;
    let ocr_timeout_secs = get_ocr_timeout_secs();

    let state = app.state::<AppState>();

    // Cancel any lingering previous job (defensive)
    {
        let mut job_lock = state.ocr_job.lock().await;
        if let Some(old_handle) = job_lock.take() {
            cancel_job_handle(old_handle).await;
        }
    }

    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    let (sidecar_path, runtime_dir) = resolve_sidecar_path(&resource_dir);

    if !sidecar_path.exists() {
        return Err(format!("OCR sidecar not found at: {:?}", sidecar_path));
    }

    #[cfg(debug_assertions)]
    {
        if let Some(ref dir) = runtime_dir {
            eprintln!("OCR runtime dir: {}", dir.display());
        }
        eprintln!("OCR sidecar executable: {}", sidecar_path.display());
    }

    if is_base64 {
        return Err(
            "OCR sidecar is path-only. Pass a stored CAS path instead of base64 data.".to_string(),
        );
    }

    let resolved_image_path = resolve_attachment_path_internal(&image_data)?;

    let mut rec_model_dir_override: Option<String> = None;
    if let Some(name) = model_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let manager = ModelManager::new().map_err(|e| e.to_string())?;
        if manager.is_model_installed(name) {
            let model_dir = manager.get_model_dir(name);
            rec_model_dir_override = Some(model_dir.to_string_lossy().to_string());
        }
    }

    let mut cmd = tokio::process::Command::new(&sidecar_path);
    cmd.arg(&resolved_image_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(ref rec_model_dir) = rec_model_dir_override {
        cmd.arg("--rec-model-dir").arg(rec_model_dir);
    }

    if let Some(ref dir) = runtime_dir {
        cmd.current_dir(dir);
    }

    cmd.env("OMP_NUM_THREADS", "1")
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("MKL_NUM_THREADS", "1")
        .env("NUMEXPR_NUM_THREADS", "1")
        .env("OMP_WAIT_POLICY", "PASSIVE");
    apply_runtime_lib_env(&mut cmd, runtime_dir.as_deref());

    #[cfg(windows)]
    {
        cmd.creation_flags(0x00004000);
    }

    #[cfg(unix)]
    {
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }

                libc::nice(10);

                #[cfg(target_os = "linux")]
                {
                    libc::syscall(
                        libc::SYS_ioprio_set,
                        1, /* IOPRIO_WHO_PROCESS */
                        0,
                        (2 << 13) | 7,
                    );
                }
                Ok(())
            });
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn OCR sidecar: {}", e))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_task = tokio::spawn(read_pipe_to_string(stdout_pipe));
    let stderr_task = tokio::spawn(read_stderr_to_string(stderr_pipe));

    {
        let mut job_lock = state.ocr_job.lock().await;
        *job_lock = Some(OcrJobHandle { child });
    }

    // Wait for the child to exit by polling through the stored handle
    let exit_status = {
        let wait_result = timeout(Duration::from_secs(ocr_timeout_secs), async {
            loop {
                let mut job_lock = state.ocr_job.lock().await;
                if let Some(ref mut handle) = *job_lock {
                    match handle.child.try_wait() {
                        Ok(Some(status)) => return Ok(status),
                        Ok(None) => {
                            drop(job_lock);
                            tokio::time::sleep(Duration::from_millis(50)).await;
                        }
                        Err(e) => return Err(format!("Failed to wait for sidecar: {}", e)),
                    }
                } else {
                    // Job was cancelled externally
                    return Err("OCR job was cancelled".to_string());
                }
            }
        })
        .await;

        // Clean up job handle
        {
            let mut job_lock = state.ocr_job.lock().await;
            *job_lock = None;
        }

        match wait_result {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout
                eprintln!(
                    "OCR sidecar timed out after {}s, terminating process",
                    ocr_timeout_secs
                );

                {
                    let mut job_lock = state.ocr_job.lock().await;
                    if let Some(handle) = job_lock.take() {
                        cancel_job_handle(handle).await;
                    }
                }

                return Err(format!(
                    "OCR timed out after {}s. The image may be too large or complex. \
                     The process has been terminated to protect system stability.",
                    ocr_timeout_secs
                ));
            }
        }
    };

    let stdout_text = stdout_task.await.unwrap_or_default();
    let stderr_text = stderr_task.await.unwrap_or_default();
    let stdout_json = extract_json_payload(&stdout_text);

    if !exit_status.success() {
        if let Some(payload) = stdout_json.as_deref() {
            if let Ok(err) = serde_json::from_str::<OcrError>(payload) {
                return Err(format!("OCR sidecar failed: {}", err.error));
            }
        }

        let stderr_trimmed = stderr_text.trim();
        let stdout_trimmed = stdout_text.trim();

        if !stderr_trimmed.is_empty() && !stdout_trimmed.is_empty() {
            return Err(format!(
                "OCR sidecar failed: {}\n{}",
                stderr_trimmed, stdout_trimmed
            ));
        }
        if !stderr_trimmed.is_empty() {
            return Err(format!("OCR sidecar failed: {}", stderr_trimmed));
        }
        if !stdout_trimmed.is_empty() {
            return Err(format!("OCR sidecar failed: {}", stdout_trimmed));
        }
        return Err("OCR sidecar failed with no error output".to_string());
    }

    let stdout_payload = stdout_json.ok_or_else(|| {
        format!(
            "Failed to parse OCR output: no JSON payload found in stdout.\nstdout={}\nstderr={}",
            stdout_text.trim(),
            stderr_text.trim()
        )
    })?;

    if let Ok(err) = serde_json::from_str::<OcrError>(&stdout_payload) {
        return Err(err.error);
    }

    let raw_results: Vec<RawOcrResult> = serde_json::from_str(&stdout_payload).map_err(|e| {
        format!(
            "Failed to parse OCR output: {} - payload={}\nstdout={}\nstderr={}",
            e,
            stdout_payload,
            stdout_text.trim(),
            stderr_text.trim()
        )
    })?;

    let results: Vec<OcrBox> = raw_results
        .into_iter()
        .map(|r| OcrBox {
            text: r.text,
            box_coords: r.bounding_box,
            confidence: r.confidence.unwrap_or(1.0),
        })
        .collect();

    Ok(results)
}

/// Cancel the currently running OCR job.
/// Kills the sidecar process and waits briefly for shutdown.
/// This is fire-and-forget from the frontend's perspective.
#[tauri::command]
pub async fn cancel_ocr_job(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let handle = {
        let mut job_lock = state.ocr_job.lock().await;
        job_lock.take()
    };

    if let Some(handle) = handle {
        cancel_job_handle(handle).await;
    }

    Ok(())
}

fn get_target_triple() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(target_os = "macos")]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(target_os = "linux")]
    {
        "x86_64-unknown-linux-gnu"
    }
}

fn get_sidecar_executable_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "ocr-engine.exe"
    }
    #[cfg(target_os = "macos")]
    {
        "ocr-engine"
    }
    #[cfg(target_os = "linux")]
    {
        "ocr-engine"
    }
}

fn get_fallback_sidecar_name() -> String {
    #[cfg(target_os = "windows")]
    {
        "ocr-engine.exe".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "ocr-engine".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "ocr-engine-x86_64-unknown-linux-gnu".to_string()
    }
}

fn get_runtime_dir_name() -> String {
    format!("paddle-ocr-{}", get_target_triple())
}

fn resolve_sidecar_path(resource_dir: &Path) -> (PathBuf, Option<PathBuf>) {
    let runtime_dir_name = get_runtime_dir_name();
    let runtime_candidates = [
        resource_dir.join("binaries").join(&runtime_dir_name),
        resource_dir.join(&runtime_dir_name),
        resource_dir
            .join("resources")
            .join("binaries")
            .join(&runtime_dir_name),
    ];

    for runtime_dir in runtime_candidates {
        let sidecar = runtime_dir.join(get_sidecar_executable_name());
        if sidecar.exists() {
            return (sidecar, Some(runtime_dir));
        }
    }

    // Backward-compatible fallback for onefile/externalBin builds.
    let fallback_name = get_fallback_sidecar_name();
    let fallback_candidates = [
        resource_dir.join("binaries").join(&fallback_name),
        resource_dir.join(&fallback_name),
    ];
    for sidecar in fallback_candidates {
        if sidecar.exists() {
            return (sidecar, None);
        }
    }

    (
        resource_dir.join("binaries").join(fallback_name),
        Some(resource_dir.join("binaries").join(runtime_dir_name)),
    )
}
