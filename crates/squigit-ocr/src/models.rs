// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use flate2::read::GzDecoder;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tar::Archive;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::network::{NetworkStatus, PeerNetworkMonitor};

macro_rules! model_log {
    ($($argument:tt)*) => {
        if std::env::var_os("SQUIGIT_TUI_ACTIVE").is_none() {
            eprintln!($($argument)*);
        }
    };
}

pub const DEFAULT_OCR_MODEL_ID: &str = "pp-ocr-v5-en";

pub trait ModelStorage: Send + Sync {
    fn models_dir(&self) -> PathBuf;
    fn ensure(&self) -> anyhow::Result<()>;
    fn selected_model(&self) -> anyhow::Result<String>;
    fn select_model(&self, model_id: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub struct OcrModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub lang: &'static str,
    pub size: &'static str,
    pub download_url: &'static str,
    hf_repo: &'static str,
}

pub const OCR_MODELS: &[OcrModelDefinition] = &[
    OcrModelDefinition {
        id: "pp-ocr-v5-en",
        name: "PP-OCR-V5 English",
        lang: "en",
        size: "~16 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/en_PP-OCRv5_mobile_rec_infer.tar",
        hf_repo: "PaddlePaddle/en_PP-OCRv5_mobile_rec",
    },
    OcrModelDefinition {
        id: "pp-ocr-v5-latin",
        name: "PP-OCR-V5 Latin",
        lang: "la",
        size: "~17 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/latin_PP-OCRv5_mobile_rec_infer.tar",
        hf_repo: "PaddlePaddle/latin_PP-OCRv5_mobile_rec",
    },
    OcrModelDefinition {
        id: "pp-ocr-v5-cyrillic",
        name: "PP-OCR-V5 Cyrillic",
        lang: "ru",
        size: "~17 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/cyrillic_PP-OCRv5_mobile_rec_infer.tar",
        hf_repo: "PaddlePaddle/cyrillic_PP-OCRv5_mobile_rec",
    },
    OcrModelDefinition {
        id: "pp-ocr-v5-korean",
        name: "PP-OCR-V5 Korean",
        lang: "ko",
        size: "~18 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/korean_PP-OCRv5_mobile_rec_infer.tar",
        hf_repo: "PaddlePaddle/korean_PP-OCRv5_mobile_rec",
    },
    OcrModelDefinition {
        id: "pp-ocr-v5-cjk",
        name: "PP-OCR-V5 CJK",
        lang: "ch",
        size: "~83 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/PP-OCRv5_server_rec_infer.tar",
        hf_repo: "PaddlePaddle/PP-OCRv5_server_rec",
    },
    OcrModelDefinition {
        id: "pp-ocr-v5-devanagari",
        name: "PP-OCR-V5 Devanagari",
        lang: "hi",
        size: "~17 MB",
        download_url: "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/devanagari_PP-OCRv5_mobile_rec_infer.tar",
        hf_repo: "PaddlePaddle/devanagari_PP-OCRv5_mobile_rec",
    },
];

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("App data error: {0:#}")]
    AppData(#[from] anyhow::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Extraction error: {0}")]
    Extraction(String),
    #[error("Unsupported OCR model id: {0}")]
    UnsupportedModelId(String),
    #[error("OCR model download preflight failed: {0}")]
    Preflight(String),
    #[error("Download cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, ModelError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct DownloadProgressPayload {
    pub id: String,
    pub progress: u8,
    pub loaded: u64,
    pub total: u64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDownloadJobSnapshot {
    pub id: String,
    pub status: String,
    pub progress: u8,
    pub loaded: u64,
    pub total: u64,
    pub error: Option<String>,
}

impl ModelDownloadJobSnapshot {
    fn checking(id: String) -> Self {
        Self {
            id,
            status: "checking_internet".to_string(),
            progress: 0,
            loaded: 0,
            total: 0,
            error: None,
        }
    }
}

#[derive(Deserialize)]
struct OfficialModelSnapshot {
    id: String,
}

fn canonical_ocr_model_id(model_id: &str) -> &str {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return DEFAULT_OCR_MODEL_ID;
    }
    model_id
}

fn ocr_model_definition(model_id: &str) -> Option<&'static OcrModelDefinition> {
    let canonical_id = canonical_ocr_model_id(model_id);
    OCR_MODELS.iter().find(|model| model.id == canonical_id)
}

fn supported_ocr_model_id(model_id: &str) -> Option<&'static str> {
    ocr_model_definition(model_id).map(|model| model.id)
}

fn has_model_graph_file(model_dir: &Path) -> bool {
    model_dir.join("inference.pdmodel").exists() || model_dir.join("inference.json").exists()
}

fn is_model_dir_ready(model_dir: &Path) -> bool {
    has_model_graph_file(model_dir) && model_dir.join("inference.pdiparams").exists()
}

fn hf_repo_for_model_id(model_id: &str) -> Option<&'static str> {
    ocr_model_definition(model_id).map(|model| model.hf_repo)
}

fn build_archive_candidates(model_id: &str) -> Vec<String> {
    ocr_model_definition(model_id)
        .map(|model| vec![model.download_url.to_string()])
        .unwrap_or_default()
}

fn is_active_download_status(status: &str) -> bool {
    status.starts_with("checking_")
        || matches!(
            status,
            "downloading" | "paused" | "extracting" | "cancelling"
        )
}

fn terminal_progress_bar(progress: u8) -> String {
    const WIDTH: usize = 32;
    let filled = usize::from(progress).saturating_mul(WIDTH) / 100;
    format!("{}{}", "█".repeat(filled), "░".repeat(WIDTH - filled))
}

#[derive(Clone)]
pub struct ModelManager {
    data: Arc<dyn ModelStorage>,
    models_dir: PathBuf,
    cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    network_monitor: Arc<PeerNetworkMonitor>,
    download_jobs: Arc<Mutex<HashMap<String, ModelDownloadJobSnapshot>>>,
    updates: watch::Sender<Vec<ModelDownloadJobSnapshot>>,
}

impl ModelManager {
    pub fn new(data: impl ModelStorage + 'static) -> Self {
        let models_dir = data.models_dir();
        let network_monitor = Arc::new(PeerNetworkMonitor::new());
        Self {
            data: Arc::new(data),
            models_dir,
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            network_monitor,
            download_jobs: Arc::new(Mutex::new(HashMap::new())),
            updates: watch::channel(Vec::new()).0,
        }
    }

    pub fn subscribe_downloads(&self) -> watch::Receiver<Vec<ModelDownloadJobSnapshot>> {
        self.updates.subscribe()
    }

    fn publish_downloads(&self) {
        self.updates.send_replace(self.download_jobs_snapshot());
    }

    pub fn selected_model(&self) -> Result<String> {
        let selected = self.data.selected_model()?;
        if selected == DEFAULT_OCR_MODEL_ID || self.is_model_installed(&selected) {
            Ok(selected)
        } else {
            Ok(DEFAULT_OCR_MODEL_ID.to_string())
        }
    }

    pub fn select_model(&self, model_id: &str) -> Result<()> {
        let id = supported_ocr_model_id(model_id)
            .ok_or_else(|| ModelError::UnsupportedModelId(model_id.to_string()))?;
        if id != DEFAULT_OCR_MODEL_ID && !self.is_model_installed(id) {
            return Err(ModelError::Preflight(
                "Download this model before selecting it".to_string(),
            ));
        }
        self.data.select_model(id)?;
        Ok(())
    }

    pub fn start_monitor(&self) {
        self.network_monitor.start_monitor();
    }

    pub fn get_model_dir(&self, model_id: &str) -> PathBuf {
        if model_id.is_empty() {
            return self.models_dir.clone();
        }

        let canonical_id = canonical_ocr_model_id(model_id);
        self.models_dir.join(canonical_id)
    }

    fn get_temp_file_path(&self, model_id: &str) -> PathBuf {
        let canonical_id = canonical_ocr_model_id(model_id);
        self.models_dir.join(format!("temp_{}.tar", canonical_id))
    }

    pub fn is_model_installed(&self, model_id: &str) -> bool {
        let Some(canonical_id) = supported_ocr_model_id(model_id) else {
            return false;
        };
        let dir = self.get_model_dir(canonical_id);
        dir.exists() && is_model_dir_ready(&dir)
    }

    pub fn list_downloaded_models(&self) -> Result<Vec<String>> {
        self.data.ensure()?;
        let mut models = Vec::new();

        if self.models_dir.exists() {
            for entry in fs::read_dir(&self.models_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() && is_model_dir_ready(&path) {
                    if let Some(name) = path.file_name() {
                        let model_id = name.to_string_lossy();
                        if supported_ocr_model_id(&model_id).is_some() {
                            models.push(model_id.to_string());
                        }
                    }
                }
            }
        }

        Ok(models)
    }

    pub fn download_jobs_snapshot(&self) -> Vec<ModelDownloadJobSnapshot> {
        let Ok(jobs) = self.download_jobs.lock() else {
            return Vec::new();
        };
        let mut snapshots = jobs.values().cloned().collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.id.cmp(&right.id));
        snapshots
    }

    pub fn start_download(&self, model_id: &str) -> Result<()> {
        self.data.ensure()?;
        let canonical_id = supported_ocr_model_id(model_id)
            .ok_or_else(|| ModelError::UnsupportedModelId(model_id.to_string()))?
            .to_string();
        {
            let mut jobs = self.download_jobs.lock().map_err(|_| {
                ModelError::Preflight("model download state is unavailable".to_string())
            })?;
            if jobs
                .get(&canonical_id)
                .is_some_and(|job| is_active_download_status(&job.status))
            {
                return Ok(());
            }
            jobs.insert(
                canonical_id.clone(),
                ModelDownloadJobSnapshot::checking(canonical_id.clone()),
            );
        }
        self.publish_downloads();
        self.cancel_active_transfer(&canonical_id);
        let cancel_token = CancellationToken::new();
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            tokens.insert(canonical_id.clone(), cancel_token.clone());
        }
        model_log!(
            "[squigit-ocr:models] {} preflight 1/3: checking internet availability",
            canonical_id
        );

        let worker_manager = self.clone();
        let worker_id = canonical_id.clone();
        let worker = tokio::spawn(async move {
            worker_manager
                .run_managed_download(worker_id, cancel_token)
                .await;
        });
        let watcher_manager = self.clone();
        tokio::spawn(async move {
            let Err(error) = worker.await else {
                return;
            };
            let message = format!("model download worker crashed: {error}");
            model_log!("[squigit-ocr:models] {canonical_id} failed\n{message}");
            watcher_manager.set_download_failure(&canonical_id, message);
        });
        Ok(())
    }

    async fn run_managed_download(&self, model_id: String, cancel_token: CancellationToken) {
        let progress_manager = self.clone();
        let result = self
            .download_and_extract_with_token(&model_id, cancel_token.clone(), move |payload| {
                progress_manager.record_download_progress(payload);
            })
            .await;
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            tokens.remove(&model_id);
        }
        let result = if cancel_token.is_cancelled() {
            Err(ModelError::Cancelled)
        } else {
            result
        };
        match result {
            Ok(path) => {
                self.record_download_progress(DownloadProgressPayload {
                    id: model_id.clone(),
                    progress: 100,
                    loaded: 0,
                    total: 0,
                    status: "downloaded".to_string(),
                });
                model_log!(
                    "[squigit-ocr:models] {} installed at {}",
                    model_id,
                    path.display()
                );
            }
            Err(ModelError::Cancelled) => {
                self.record_download_progress(DownloadProgressPayload {
                    id: model_id,
                    progress: 0,
                    loaded: 0,
                    total: 0,
                    status: "cancelled".to_string(),
                });
            }
            Err(error) => {
                let message = error.to_string();
                model_log!("[squigit-ocr:models] {model_id} failed\n{message}");
                self.set_download_failure(&model_id, message);
            }
        }
    }

    fn set_download_failure(&self, model_id: &str, error: String) {
        if let Ok(mut jobs) = self.download_jobs.lock() {
            let job = jobs
                .entry(model_id.to_string())
                .or_insert_with(|| ModelDownloadJobSnapshot::checking(model_id.to_string()));
            job.status = "failed".to_string();
            job.error = Some(error);
        }
        self.publish_downloads();
    }

    fn record_download_progress(&self, payload: DownloadProgressPayload) {
        let previous = if let Ok(mut jobs) = self.download_jobs.lock() {
            if jobs
                .get(&payload.id)
                .is_some_and(|job| matches!(job.status.as_str(), "cancelling" | "cancelled"))
                && payload.status != "cancelled"
            {
                return;
            }
            let previous = jobs.get(&payload.id).cloned();
            jobs.insert(
                payload.id.clone(),
                ModelDownloadJobSnapshot {
                    id: payload.id.clone(),
                    status: payload.status.clone(),
                    progress: payload.progress,
                    loaded: payload.loaded,
                    total: payload.total,
                    error: None,
                },
            );
            previous
        } else {
            None
        };

        self.publish_downloads();
        let status_changed = previous
            .as_ref()
            .is_none_or(|job| job.status != payload.status);
        let progress_changed = previous
            .as_ref()
            .is_none_or(|job| job.progress != payload.progress);
        if status_changed {
            match payload.status.as_str() {
                "internet_available" => {
                    model_log!("[squigit-ocr:models] {} preflight 1/3: passed", payload.id)
                }
                "checking_installation" => model_log!(
                    "[squigit-ocr:models] {} preflight 2/3: checking user model path",
                    payload.id
                ),
                "model_not_installed" => model_log!(
                    "[squigit-ocr:models] {} preflight 2/3: passed (not installed)",
                    payload.id
                ),
                "already_installed" => model_log!(
                    "[squigit-ocr:models] {} preflight 2/3: already installed",
                    payload.id
                ),
                "checking_availability" => model_log!(
                    "[squigit-ocr:models] {} preflight 3/3: reading PaddlePaddle model snapshot",
                    payload.id
                ),
                "model_available" => {
                    model_log!("[squigit-ocr:models] {} preflight 3/3: passed", payload.id)
                }
                "extracting" => {
                    model_log!("[squigit-ocr:models] {} extracting", payload.id)
                }
                "cancelled" => {
                    model_log!("[squigit-ocr:models] {} cancelled", payload.id)
                }
                _ => {}
            }
        }
        if payload.status == "downloading" && progress_changed {
            let first = previous
                .as_ref()
                .filter(|job| job.status == "downloading" && job.progress < payload.progress)
                .map_or(payload.progress, |job| job.progress.saturating_add(1));
            for progress in first..=payload.progress {
                model_log!(
                    "[squigit-ocr:models] {} [{}] {:3}%",
                    payload.id,
                    terminal_progress_bar(progress),
                    progress
                );
            }
        }
    }

    fn cancel_active_transfer(&self, model_id: &str) {
        let canonical_id = canonical_ocr_model_id(model_id).to_string();
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            if let Some(token) = tokens.remove(&canonical_id) {
                token.cancel();
            }
        }
    }

    pub fn cancel_download(&self, model_id: &str) {
        let canonical_id = canonical_ocr_model_id(model_id).to_string();
        self.cancel_active_transfer(&canonical_id);
        if let Ok(mut jobs) = self.download_jobs.lock() {
            if let Some(job) = jobs.get_mut(&canonical_id) {
                if is_active_download_status(&job.status) {
                    job.status = "cancelling".to_string();
                }
            }
        }
        self.publish_downloads();
        model_log!("[squigit-ocr:models] {canonical_id} cancel requested");
    }

    pub fn trash_downloaded_model(&self, model_id: &str) -> Result<()> {
        let canonical_id = supported_ocr_model_id(model_id)
            .ok_or_else(|| ModelError::UnsupportedModelId(model_id.to_string()))?;
        if canonical_id == DEFAULT_OCR_MODEL_ID {
            return Err(ModelError::Preflight(
                "The bundled English model cannot be deleted".to_string(),
            ));
        }
        if self.selected_model()? == canonical_id {
            self.select_model(DEFAULT_OCR_MODEL_ID)?;
        }
        self.cancel_download(canonical_id);

        let temp_file_path = self.get_temp_file_path(canonical_id);
        if temp_file_path.exists() {
            fs::remove_file(&temp_file_path)?;
        }
        if let Ok(mut jobs) = self.download_jobs.lock() {
            jobs.remove(canonical_id);
        }

        let model_dir = self.get_model_dir(canonical_id);
        if !model_dir.exists() {
            return Ok(());
        }

        if model_dir.is_dir() {
            fs::remove_dir_all(model_dir)?;
        } else {
            fs::remove_file(model_dir)?;
        }

        self.publish_downloads();
        Ok(())
    }

    async fn download_and_extract_with_token<F>(
        &self,
        model_id: &str,
        cancel_token: CancellationToken,
        mut on_progress: F,
    ) -> Result<PathBuf>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        let model = ocr_model_definition(model_id)
            .ok_or_else(|| ModelError::UnsupportedModelId(model_id.to_string()))?;
        let canonical_id = model.id.to_string();
        let target_dir = self.get_model_dir(&canonical_id);
        let client = reqwest::Client::builder()
            .user_agent("squigit-ocr-model-downloader/1.0")
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        if cancel_token.is_cancelled() {
            return Err(ModelError::Cancelled);
        }

        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "checking_internet".to_string(),
        });
        tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            response = client.get("https://huggingface.co").send() => response,
        }
        .map_err(|error| ModelError::Preflight(format!("internet is unavailable: {error}")))?
        .error_for_status()
        .map_err(|error| ModelError::Preflight(format!("internet check failed: {error}")))?;
        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "internet_available".to_string(),
        });

        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "checking_installation".to_string(),
        });

        if self.is_model_installed(&canonical_id) {
            on_progress(DownloadProgressPayload {
                id: canonical_id.clone(),
                progress: 100,
                loaded: 0,
                total: 0,
                status: "already_installed".to_string(),
            });
            if let Ok(mut tokens) = self.cancellation_tokens.lock() {
                tokens.remove(&canonical_id);
            }
            return Ok(target_dir);
        }
        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "model_not_installed".to_string(),
        });

        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "checking_availability".to_string(),
        });
        let official_request = client.get("https://huggingface.co/api/models").query(&[
            ("author", "PaddlePaddle"),
            ("search", "PP-OCRv5"),
            ("limit", "100"),
        ]);
        let official_response = tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            response = official_request.send() => response?,
        }
        .error_for_status()?;
        let official_models = tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            snapshot = official_response.json::<Vec<OfficialModelSnapshot>>() => snapshot?,
        };
        if !official_models
            .iter()
            .any(|entry| entry.id == model.hf_repo)
        {
            return Err(ModelError::Preflight(format!(
                "target model {} is not present in the official PaddlePaddle snapshot",
                model.hf_repo
            )));
        }
        on_progress(DownloadProgressPayload {
            id: canonical_id.clone(),
            progress: 0,
            loaded: 0,
            total: 0,
            status: "model_available".to_string(),
        });

        let temp_file_path = self.get_temp_file_path(&canonical_id);
        let archive_candidates = build_archive_candidates(&canonical_id);
        let mut selected_archive_url: Option<String> = None;

        for candidate_url in archive_candidates {
            let mut attempts = 0u8;
            loop {
                if cancel_token.is_cancelled() {
                    if let Ok(mut tokens) = self.cancellation_tokens.lock() {
                        tokens.remove(&canonical_id);
                    }

                    let _ = fs::remove_file(&temp_file_path);
                    return Err(ModelError::Cancelled);
                }

                let net_state = self.network_monitor.get_state();
                let metadata = fs::metadata(&temp_file_path).ok();
                let current_bytes = metadata.map(|m| m.len()).unwrap_or(0);

                let status_str = if net_state.status == NetworkStatus::Offline {
                    // Hint only: do not hard-stop retries on DNS/firewall-restricted networks.
                    "paused"
                } else {
                    "downloading"
                };
                on_progress(DownloadProgressPayload {
                    id: canonical_id.clone(),
                    progress: 0,
                    loaded: current_bytes,
                    total: 0,
                    status: status_str.to_string(),
                });

                match self
                    .download_chunk_loop(
                        &client,
                        &candidate_url,
                        &canonical_id,
                        &temp_file_path,
                        &mut on_progress,
                        &cancel_token,
                    )
                    .await
                {
                    Ok(_) => {
                        selected_archive_url = Some(candidate_url.clone());
                        break;
                    }
                    Err(ModelError::Cancelled) => {
                        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
                            tokens.remove(&canonical_id);
                        }

                        let _ = fs::remove_file(&temp_file_path);
                        return Err(ModelError::Cancelled);
                    }
                    Err(e) => {
                        attempts = attempts.saturating_add(1);
                        model_log!(
                            "Download attempt failed ({}, try {}/3): {}",
                            candidate_url,
                            attempts,
                            e
                        );

                        on_progress(DownloadProgressPayload {
                            id: canonical_id.clone(),
                            progress: 0,
                            loaded: current_bytes,
                            total: 0,
                            status: "paused".to_string(),
                        });

                        if attempts >= 3 {
                            let _ = fs::remove_file(&temp_file_path);
                            break;
                        }
                        tokio::select! {
                            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
                            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {},
                        }
                    }
                }
            }

            if selected_archive_url.is_some() {
                break;
            }
        }

        if let Some(chosen_url) = selected_archive_url {
            if let Ok(mut tokens) = self.cancellation_tokens.lock() {
                tokens.remove(&canonical_id);
            }

            model_log!("Extracting model {}...", canonical_id);
            return self
                .perform_extraction(
                    &chosen_url,
                    &canonical_id,
                    &temp_file_path,
                    &target_dir,
                    &mut on_progress,
                )
                .await;
        }

        model_log!(
            "Archive download failed for {}. Falling back to direct model file download...",
            canonical_id
        );
        let fallback_result = self
            .download_direct_model_files(
                &client,
                &canonical_id,
                &target_dir,
                &mut on_progress,
                &cancel_token,
            )
            .await;
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            tokens.remove(&canonical_id);
        }
        let _ = fs::remove_file(&temp_file_path);
        fallback_result
    }

    async fn download_file_to_path<F>(
        &self,
        client: &reqwest::Client,
        url: &str,
        target_path: &Path,
        cancel_token: &CancellationToken,
        mut on_bytes: F,
    ) -> Result<u64>
    where
        F: FnMut(u64) + Send,
    {
        let response = tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            response = client.get(url).send() => response?,
        }
        .error_for_status()?;
        let mut file = File::create(target_path).await?;
        let mut stream = response.bytes_stream();
        let mut written = 0u64;
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
                item = stream.next() => match item {
                    Some(chunk) => {
                        let chunk = chunk?;
                        file.write_all(&chunk).await?;
                        written += chunk.len() as u64;
                        on_bytes(written);
                    }
                    None => break,
                }
            }
        }
        file.flush().await?;
        Ok(written)
    }

    async fn download_direct_model_files<F>(
        &self,
        client: &reqwest::Client,
        model_id: &str,
        target_dir: &Path,
        on_progress: &mut F,
        cancel_token: &CancellationToken,
    ) -> Result<PathBuf>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        let repo = hf_repo_for_model_id(model_id)
            .ok_or_else(|| ModelError::UnsupportedModelId(model_id.to_string()))?;
        const HF_BASES: [&str; 2] = ["https://huggingface.co", "https://hf-mirror.com"];
        const REQUIRED_FILES: [&str; 3] =
            ["inference.json", "inference.pdiparams", "inference.yml"];
        let mut errors = Vec::new();
        for base in HF_BASES {
            if cancel_token.is_cancelled() {
                return Err(ModelError::Cancelled);
            }
            let staging = tempfile::Builder::new()
                .prefix(".download-")
                .tempdir_in(&self.models_dir)?;
            let mut total = Some(0u64);
            for name in REQUIRED_FILES {
                let url = format!("{base}/{repo}/resolve/main/{name}");
                let response = tokio::select! {
                    _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
                    response = client.head(url).send() => response,
                };
                let length = response
                    .ok()
                    .filter(|value| value.status().is_success())
                    .and_then(|value| value.content_length());
                total = total.zip(length).map(|(sum, length)| sum + length);
            }
            let total = total.unwrap_or(0);
            let mut loaded = 0u64;
            let mut success = true;
            for name in REQUIRED_FILES {
                let url = format!("{base}/{repo}/resolve/main/{name}");
                let path = staging.path().join(name);
                let result = self
                    .download_file_to_path(client, &url, &path, cancel_token, |written| {
                        let loaded = loaded + written;
                        on_progress(DownloadProgressPayload {
                            id: model_id.to_string(),
                            progress: if total > 0 {
                                ((loaded.saturating_mul(100) / total).min(99)) as u8
                            } else {
                                0
                            },
                            loaded,
                            total,
                            status: "downloading".to_string(),
                        });
                    })
                    .await;
                match result {
                    Ok(written) => loaded += written,
                    Err(ModelError::Cancelled) => return Err(ModelError::Cancelled),
                    Err(error) => {
                        errors.push(format!("{url}: {error}"));
                        success = false;
                        break;
                    }
                }
            }
            if success && is_model_dir_ready(staging.path()) {
                if cancel_token.is_cancelled() {
                    return Err(ModelError::Cancelled);
                }
                on_progress(DownloadProgressPayload {
                    id: model_id.to_string(),
                    progress: 100,
                    loaded,
                    total,
                    status: "extracting".to_string(),
                });
                if target_dir.exists() {
                    fs::remove_dir_all(target_dir)?;
                }
                fs::rename(staging.path(), target_dir)?;
                return Ok(target_dir.to_path_buf());
            }
        }
        Err(ModelError::Extraction(format!(
            "Model download failed for {model_id}:\n{}",
            errors.join("\n")
        )))
    }

    async fn download_chunk_loop<F>(
        &self,
        client: &reqwest::Client,
        url: &str,
        model_id: &str,
        temp_file_path: &Path,
        on_progress: &mut F,
        cancel_token: &CancellationToken,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        let mut downloaded_bytes = 0u64;
        let mut file;

        if temp_file_path.exists() {
            let metadata = fs::metadata(temp_file_path)?;
            downloaded_bytes = metadata.len();
            file = File::options().append(true).open(temp_file_path).await?;
        } else {
            file = File::create(temp_file_path).await?;
        }

        let head = tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            response = client.head(url).send() => response,
        };
        let (mut total_size, can_resume) = match head {
            Ok(head_resp) if head_resp.status().is_success() => {
                let total_size = head_resp
                    .headers()
                    .get(CONTENT_LENGTH)
                    .and_then(|ct| ct.to_str().ok())
                    .and_then(|ct| ct.parse::<u64>().ok())
                    .unwrap_or(0);
                let can_resume = head_resp
                    .headers()
                    .get(ACCEPT_RANGES)
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.to_ascii_lowercase().contains("bytes"))
                    .unwrap_or(false);
                (total_size, can_resume)
            }
            Ok(head_resp) => {
                model_log!(
                    "HEAD {} returned {}; continuing without resume metadata",
                    url,
                    head_resp.status()
                );
                (0, false)
            }
            Err(err) => {
                model_log!("HEAD {} failed ({}); continuing with direct GET", url, err);
                (0, false)
            }
        };

        if total_size > 0 && downloaded_bytes >= total_size {
            return Ok(());
        }

        if downloaded_bytes > 0 && !can_resume {
            drop(file);
            let _ = fs::remove_file(temp_file_path);
            downloaded_bytes = 0;
            file = File::create(temp_file_path).await?;
        }

        let request = if downloaded_bytes > 0 {
            client
                .get(url)
                .header(RANGE, format!("bytes={downloaded_bytes}-"))
        } else {
            client.get(url)
        };
        let mut response = tokio::select! {
            _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
            response = request.send() => response?,
        };

        if downloaded_bytes > 0 && response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            if total_size > 0 && downloaded_bytes >= total_size {
                return Ok(());
            }
            drop(file);
            let _ = fs::remove_file(temp_file_path);
            downloaded_bytes = 0;
            file = File::create(temp_file_path).await?;
            response = tokio::select! {
                _ = cancel_token.cancelled() => return Err(ModelError::Cancelled),
                response = client.get(url).send() => response?,
            };
        }

        if downloaded_bytes > 0 && response.status() == reqwest::StatusCode::OK {
            // Server ignored Range and returned a full body. Restart from scratch.
            drop(file);
            downloaded_bytes = 0;
            file = File::create(temp_file_path).await?;
        }

        if !response.status().is_success() {
            return Err(ModelError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        if total_size == 0 {
            total_size = response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|ct| ct.to_str().ok())
                .and_then(|ct| ct.parse::<u64>().ok())
                .map(|len| len + downloaded_bytes)
                .unwrap_or(0);
        }

        let mut stream = response.bytes_stream();

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return Err(ModelError::Cancelled);
                }
                item = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next()) => {
                    match item {
                        Ok(Some(chunk_result)) => {
                            let chunk = chunk_result?;
                            file.write_all(&chunk).await?;
                            downloaded_bytes += chunk.len() as u64;

                            {
                                let progress = if total_size > 0 { ((downloaded_bytes.saturating_mul(100) / total_size).min(100)) as u8 } else { 0 };
                                on_progress(DownloadProgressPayload {
                                    id: model_id.to_string(),
                                    progress,
                                    loaded: downloaded_bytes,
                                    total: total_size,
                                    status: "downloading".to_string(),
                                });
                            }
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => {
                            return Err(ModelError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "Network timeout")));
                        }
                    }
                }
            }
        }

        file.flush().await?;
        Ok(())
    }

    async fn perform_extraction<F>(
        &self,
        url: &str,
        model_id: &str,
        temp_file_path: &Path,
        target_dir: &Path,
        on_progress: &mut F,
    ) -> Result<PathBuf>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        on_progress(DownloadProgressPayload {
            id: model_id.to_string(),
            progress: 100,
            loaded: 0,
            total: 0,
            status: "extracting".to_string(),
        });

        let staging = tempfile::Builder::new()
            .prefix(".extract-")
            .tempdir_in(&self.models_dir)?;

        let tar_file = fs::File::open(temp_file_path)?;
        let mut archive = Archive::new(tar_file);

        if url.ends_with(".tar") {
            self.extract_archive(&mut archive, staging.path())?;
        } else if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            let tar_file = fs::File::open(temp_file_path)?;
            let tar = GzDecoder::new(tar_file);
            let mut archive = Archive::new(tar);
            self.extract_archive(&mut archive, staging.path())?;
        } else if self.extract_archive(&mut archive, staging.path()).is_err() {
            fs::remove_dir_all(staging.path())?;
            fs::create_dir_all(staging.path())?;
            let tar_file = fs::File::open(temp_file_path)?;
            let tar_gz = GzDecoder::new(tar_file);
            let mut archive_gz = Archive::new(tar_gz);
            self.extract_archive(&mut archive_gz, staging.path())
                .map_err(|e| ModelError::Extraction(format!("Failed to extract: {:?}", e)))?;
        }

        if target_dir.exists() {
            fs::remove_dir_all(target_dir)?;
        }
        fs::rename(staging.path(), target_dir)?;
        fs::remove_file(temp_file_path)?;
        model_log!(
            "Model {} installed successfully at {:?}",
            model_id,
            target_dir
        );
        Ok(target_dir.to_path_buf())
    }

    fn extract_archive<R: std::io::Read>(
        &self,
        archive: &mut Archive<R>,
        target_dir: &Path,
    ) -> Result<()> {
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            if !entry.header().entry_type().is_file() {
                continue;
            }

            if let Some(name) = path.file_name() {
                // Flatten archive paths into model dir; keeps required metadata files
                // (e.g. inference.yml / inference.json) from PP3 model bundles.
                let dest = target_dir.join(name);
                entry.unpack(dest)?;
            }
        }

        if !has_model_graph_file(target_dir) {
            return Err(ModelError::Extraction(
                "Archive did not contain inference.pdmodel or inference.json".to_string(),
            ));
        }

        if !target_dir.join("inference.pdiparams").exists() {
            return Err(ModelError::Extraction(
                "Archive did not contain inference.pdiparams".to_string(),
            ));
        }

        Ok(())
    }
}
