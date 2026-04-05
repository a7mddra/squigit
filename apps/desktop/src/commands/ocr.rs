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
use std::sync::OnceLock;
use tauri::Manager;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[cfg(unix)]
use std::io;

use crate::brain::provider::gemini::attachments::paths::resolve_attachment_path_internal;
use crate::services::models::ModelManager;
use crate::state::{AppState, OcrJobHandle};

/// Maximum wall-clock time for a single OCR job (seconds).
const OCR_TIMEOUT_SECS_DEFAULT: u64 = 120;

/// Global mutex to ensure only one OCR job runs at a time.
/// Prevents concurrent calls from compounding CPU pressure.
static OCR_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn ocr_lock() -> &'static Mutex<()> {
    OCR_LOCK.get_or_init(|| Mutex::new(()))
}

fn get_ocr_timeout_secs() -> u64 {
    std::env::var("SQUIGIT_OCR_TIMEOUT_SECS")
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

// run_sidecar_version moved to system.rs

#[tauri::command]
pub async fn ocr_image(
    app: tauri::AppHandle,
    image_data: String,
    is_base64: bool,
    model_name: Option<String>,
) -> Result<Vec<OcrBox>, String> {
    let _guard = ocr_lock().lock().await;
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

    // If it's a relative path ("squigit-ocr"), skip the .exists() check and let Command::new resolve it via PATH.
    // If it's absolute, check if it exists.
    if sidecar_path.is_absolute() && !sidecar_path.exists() {
        return Err("ERR_MISSING_OCR_PACKAGE".to_string());
    }

    // Verify sidecar version compatibility
    check_ocr_version(&sidecar_path)?;

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
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
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

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("ERR_MISSING_OCR_PACKAGE".to_string());
        }
        Err(e) => return Err(format!("Failed to spawn OCR sidecar: {}", e)),
    };

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



fn get_ocr_target_triple() -> &'static str {
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

pub(crate) fn resolve_sidecar_path(_resource_dir: &Path) -> (PathBuf, Option<PathBuf>) {
    let name = if cfg!(windows) { "squigit-ocr.exe" } else { "squigit-ocr" };

    // 1. PATH (installed via winget/brew/apt/dnf)
    if let Ok(path) = which::which(name) {
        return (path, None);
    }

    // 2. macOS GUI Fallback
    #[cfg(target_os = "macos")]
    {
        let brew_arm = PathBuf::from("/opt/homebrew/bin/squigit-ocr");
        let brew_intel = PathBuf::from("/usr/local/bin/squigit-ocr");
        if brew_arm.exists() { return (brew_arm, None); }
        if brew_intel.exists() { return (brew_intel, None); }
    }

    // 3. Windows GUI (Winget) Fallback
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_path = PathBuf::from(local_app_data)
                .join("Microsoft").join("WindowsApps").join("squigit-ocr.exe");
            if winget_path.exists() { return (winget_path, None); }
        }
    }

    // 4. Packaged runtime dir (legacy / transition case)
    let host_triple = get_ocr_target_triple();
    let runtime = _resource_dir.join("binaries").join(format!("paddle-ocr-{}", host_triple));
    let candidate = runtime.join(name);
    if candidate.exists() {
        return (candidate, Some(runtime.clone()));
    }

    // 5. Dev mode fallback
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(target_dir) = current_exe.parent().and_then(|p| p.parent()) {
            let debug_runtime = target_dir.join("debug").join("binaries").join(format!("paddle-ocr-{}", host_triple));
            let debug_candidate = debug_runtime.join(name);
            if debug_candidate.exists() {
                return (debug_candidate, Some(debug_runtime));
            }
        }
    }

    (PathBuf::from(name), None)
}

const REQUIRED_OCR_VERSION: &str = "1.2.0";

fn check_ocr_version(sidecar_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new(sidecar_path)
        .arg("--version")
        .output()
        .map_err(|_| "ERR_MISSING_OCR_PACKAGE".to_string())?;

    if !output.status.success() {
        return Err("ERR_MISSING_OCR_PACKAGE".to_string());
    }

    let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parsed: Vec<u32> = version_str.split('.').filter_map(|s| s.parse().ok()).collect();
    let req: Vec<u32> = REQUIRED_OCR_VERSION.split('.').filter_map(|s| s.parse().ok()).collect();
    
    if parsed.len() == 3 && req.len() == 3 {
        // Strict lock logic: Major must act as lock, minor/patch >=
        if parsed[0] != req[0] || parsed[1] < req[1] || (parsed[1] == req[1] && parsed[2] < req[2]) {
            return Err("ERR_OUTDATED_OCR_PACKAGE".to_string());
        }
        return Ok(());
    }
    
    if version_str != REQUIRED_OCR_VERSION {
        return Err("ERR_OUTDATED_OCR_PACKAGE".to_string());
    }

    Ok(())
}
