// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_chat_storage::{ChatStorage, OcrRegion, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, OnceLock};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

#[cfg(unix)]
use std::io;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Maximum wall-clock time for a single OCR job (seconds).
const OCR_TIMEOUT_SECS_DEFAULT: u64 = 120;

/// Global mutex to ensure only one OCR job runs at a time.
/// Prevents concurrent calls from compounding CPU pressure.
static OCR_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn ocr_lock() -> &'static Mutex<()> {
    OCR_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBox {
    pub text: String,
    pub box_coords: Vec<Vec<f64>>,
    #[serde(default)]
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct OcrExecutionResult {
    pub boxes: Vec<OcrBox>,
    pub raw_json: String,
    pub raw_text: String,
}

#[derive(Debug, Clone)]
pub struct OcrRequest {
    pub sidecar_path: PathBuf,
    pub runtime_dir: Option<PathBuf>,
    pub image_path: PathBuf,
    pub rec_model_dir_override: Option<PathBuf>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Error)]
pub enum OcrRuntimeError {
    #[error("ERR_MISSING_OCR_PACKAGE")]
    MissingPackage,
    #[error("OCR job was cancelled")]
    Cancelled,
    #[error("{0}")]
    Message(String),
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

struct OcrJobHandle {
    child: tokio::process::Child,
}

#[derive(Default)]
pub struct OcrRuntime {
    job: Arc<Mutex<Option<OcrJobHandle>>>,
}

impl OcrRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run(&self, request: OcrRequest) -> Result<OcrExecutionResult, OcrRuntimeError> {
        let _guard = ocr_lock().lock().await;
        let ocr_timeout_secs = request.timeout_secs.unwrap_or_else(get_ocr_timeout_secs);

        // Cancel any lingering previous job (defensive).
        self.cancel_current_job().await?;

        let mut cmd = tokio::process::Command::new(&request.sidecar_path);
        cmd.arg(&request.image_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref rec_model_dir) = request.rec_model_dir_override {
            cmd.arg("--rec-model-dir").arg(rec_model_dir);
        }

        if let Some(ref dir) = request.runtime_dir {
            cmd.current_dir(dir);
        }

        cmd.env("OMP_NUM_THREADS", "1")
            .env("OPENBLAS_NUM_THREADS", "1")
            .env("MKL_NUM_THREADS", "1")
            .env("NUMEXPR_NUM_THREADS", "1")
            .env("OMP_WAIT_POLICY", "PASSIVE");
        apply_runtime_lib_env(&mut cmd, request.runtime_dir.as_deref());

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
                return Err(OcrRuntimeError::MissingPackage);
            }
            Err(e) => {
                return Err(OcrRuntimeError::Message(format!(
                    "Failed to spawn OCR sidecar: {}",
                    e
                )));
            }
        };

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        let stdout_task = tokio::spawn(read_pipe_to_string(stdout_pipe));
        let stderr_task = tokio::spawn(read_stderr_to_string(stderr_pipe));

        {
            let mut job_lock = self.job.lock().await;
            *job_lock = Some(OcrJobHandle { child });
        }

        let exit_status = {
            let wait_result = timeout(Duration::from_secs(ocr_timeout_secs), async {
                loop {
                    let mut job_lock = self.job.lock().await;
                    if let Some(ref mut handle) = *job_lock {
                        match handle.child.try_wait() {
                            Ok(Some(status)) => return Ok(status),
                            Ok(None) => {
                                drop(job_lock);
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                            Err(e) => {
                                return Err(OcrRuntimeError::Message(format!(
                                    "Failed to wait for sidecar: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        return Err(OcrRuntimeError::Cancelled);
                    }
                }
            })
            .await;

            {
                let mut job_lock = self.job.lock().await;
                *job_lock = None;
            }

            match wait_result {
                Ok(Ok(status)) => status,
                Ok(Err(err)) => return Err(err),
                Err(_) => {
                    let _ = self.cancel_current_job().await;
                    return Err(OcrRuntimeError::Message(format!(
                        "OCR timed out after {}s. The image may be too large or complex. \
                         The process has been terminated to protect system stability.",
                        ocr_timeout_secs
                    )));
                }
            }
        };

        let stdout_text = stdout_task.await.unwrap_or_default();
        let stderr_text = stderr_task.await.unwrap_or_default();
        let stdout_json = extract_json_payload(&stdout_text);

        if !exit_status.success() {
            if let Some(payload) = stdout_json.as_deref() {
                if let Ok(err) = serde_json::from_str::<OcrError>(payload) {
                    return Err(OcrRuntimeError::Message(format!(
                        "OCR sidecar failed: {}",
                        err.error
                    )));
                }
            }

            let stderr_trimmed = stderr_text.trim();
            let stdout_trimmed = stdout_text.trim();

            if !stderr_trimmed.is_empty() && !stdout_trimmed.is_empty() {
                return Err(OcrRuntimeError::Message(format!(
                    "OCR sidecar failed: {}\n{}",
                    stderr_trimmed, stdout_trimmed
                )));
            }
            if !stderr_trimmed.is_empty() {
                return Err(OcrRuntimeError::Message(format!(
                    "OCR sidecar failed: {}",
                    stderr_trimmed
                )));
            }
            if !stdout_trimmed.is_empty() {
                return Err(OcrRuntimeError::Message(format!(
                    "OCR sidecar failed: {}",
                    stdout_trimmed
                )));
            }
            return Err(OcrRuntimeError::Message(
                "OCR sidecar failed with no error output".to_string(),
            ));
        }

        let stdout_payload = stdout_json.ok_or_else(|| {
            OcrRuntimeError::Message(format!(
                "Failed to parse OCR output: no JSON payload found in stdout.\nstdout={}\nstderr={}",
                stdout_text.trim(),
                stderr_text.trim()
            ))
        })?;

        if let Ok(err) = serde_json::from_str::<OcrError>(&stdout_payload) {
            return Err(OcrRuntimeError::Message(err.error));
        }

        let raw_results: Vec<RawOcrResult> =
            serde_json::from_str(&stdout_payload).map_err(|e| {
                OcrRuntimeError::Message(format!(
                    "Failed to parse OCR output: {} - payload={}\nstdout={}\nstderr={}",
                    e,
                    stdout_payload,
                    stdout_text.trim(),
                    stderr_text.trim()
                ))
            })?;

        let boxes: Vec<OcrBox> = raw_results
            .into_iter()
            .map(|r| OcrBox {
                text: r.text,
                box_coords: r.bounding_box,
                confidence: r.confidence.unwrap_or(1.0),
            })
            .collect();

        let raw_text = flatten_raw_text(&boxes);
        Ok(OcrExecutionResult {
            boxes,
            raw_json: stdout_payload,
            raw_text,
        })
    }

    pub async fn cancel_current_job(&self) -> Result<(), OcrRuntimeError> {
        let handle = {
            let mut job_lock = self.job.lock().await;
            job_lock.take()
        };

        if let Some(handle) = handle {
            cancel_job_handle(handle).await;
        }

        Ok(())
    }
}

pub fn boxes_to_storage_regions(boxes: &[OcrBox]) -> Vec<OcrRegion> {
    boxes
        .iter()
        .map(|entry| OcrRegion {
            text: entry.text.clone(),
            bbox: entry
                .box_coords
                .iter()
                .map(|point| {
                    let x = point.first().copied().unwrap_or_default().round() as i32;
                    let y = point.get(1).copied().unwrap_or_default().round() as i32;
                    vec![x, y]
                })
                .collect(),
        })
        .collect()
}

pub fn persist_boxes_to_chat_storage(
    storage: &ChatStorage,
    chat_id: &str,
    model_id: &str,
    boxes: &[OcrBox],
) -> Result<(), StorageError> {
    let regions = boxes_to_storage_regions(boxes);
    storage.save_ocr_data(chat_id, model_id, &regions)
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

fn flatten_raw_text(boxes: &[OcrBox]) -> String {
    let lines = boxes
        .iter()
        .map(|entry| entry.text.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{boxes_to_storage_regions, extract_json_payload, flatten_raw_text, OcrBox};

    #[test]
    fn json_payload_extraction_handles_noisy_stdout() {
        let raw = "booting...\n{\"error\":\"none\"}\n[{\"text\":\"hello\",\"box\":[[0,0],[1,0],[1,1],[0,1]]}]";
        let payload = extract_json_payload(raw).expect("payload");
        assert!(payload.starts_with('['));
    }

    #[test]
    fn raw_text_is_ordered_trimmed_and_newline_joined() {
        let boxes = vec![
            OcrBox {
                text: "  Hello  ".to_string(),
                box_coords: vec![],
                confidence: 1.0,
            },
            OcrBox {
                text: " ".to_string(),
                box_coords: vec![],
                confidence: 1.0,
            },
            OcrBox {
                text: "World".to_string(),
                box_coords: vec![],
                confidence: 1.0,
            },
        ];

        assert_eq!(flatten_raw_text(&boxes), "Hello\nWorld");
    }

    #[test]
    fn conversion_to_storage_regions_is_deterministic() {
        let boxes = vec![OcrBox {
            text: "A".to_string(),
            box_coords: vec![vec![1.49, 2.5], vec![3.0, 4.0]],
            confidence: 0.9,
        }];

        let regions = boxes_to_storage_regions(&boxes);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].text, "A");
        assert_eq!(regions[0].bbox, vec![vec![1, 3], vec![3, 4]]);
    }
}
