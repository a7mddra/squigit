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
use tokio_util::sync::CancellationToken;

use crate::network::{NetworkStatus, PeerNetworkMonitor};

const APP_DIR_NAME: &str = "squigit";
const DEFAULT_OCR_LANGUAGE: &str = "pp-ocr-v5-en";

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Failed to determine config directory")]
    NoConfigDir,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Extraction error: {0}")]
    Extraction(String),
    #[error("Download cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, ModelError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadProgressPayload {
    pub id: String,
    pub progress: u8,
    pub loaded: u64,
    pub total: u64,
    pub status: String,
}

fn canonical_ocr_model_id(model_id: &str) -> &str {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return DEFAULT_OCR_LANGUAGE;
    }
    model_id
}

fn has_model_graph_file(model_dir: &Path) -> bool {
    model_dir.join("inference.pdmodel").exists() || model_dir.join("inference.json").exists()
}

fn is_model_dir_ready(model_dir: &Path) -> bool {
    has_model_graph_file(model_dir) && model_dir.join("inference.pdiparams").exists()
}

fn official_archive_name_for_model_id(model_id: &str) -> Option<&'static str> {
    match canonical_ocr_model_id(model_id) {
        "pp-ocr-v5-en" => Some("en_PP-OCRv5_mobile_rec_infer.tar"),
        "pp-ocr-v5-latin" => Some("latin_PP-OCRv5_mobile_rec_infer.tar"),
        "pp-ocr-v5-cyrillic" => Some("cyrillic_PP-OCRv5_mobile_rec_infer.tar"),
        "pp-ocr-v5-korean" => Some("korean_PP-OCRv5_mobile_rec_infer.tar"),
        "pp-ocr-v5-cjk" => Some("PP-OCRv5_server_rec_infer.tar"),
        "pp-ocr-v5-devanagari" => Some("devanagari_PP-OCRv5_mobile_rec_infer.tar"),
        _ => None,
    }
}

fn hf_repo_for_model_id(model_id: &str) -> Option<&'static str> {
    match canonical_ocr_model_id(model_id) {
        "pp-ocr-v5-en" => Some("PaddlePaddle/en_PP-OCRv5_mobile_rec"),
        "pp-ocr-v5-latin" => Some("PaddlePaddle/latin_PP-OCRv5_mobile_rec"),
        "pp-ocr-v5-cyrillic" => Some("PaddlePaddle/cyrillic_PP-OCRv5_mobile_rec"),
        "pp-ocr-v5-korean" => Some("PaddlePaddle/korean_PP-OCRv5_mobile_rec"),
        "pp-ocr-v5-cjk" => Some("PaddlePaddle/PP-OCRv5_server_rec"),
        "pp-ocr-v5-devanagari" => Some("PaddlePaddle/devanagari_PP-OCRv5_mobile_rec"),
        _ => None,
    }
}

fn build_archive_candidates(primary_url: &str, model_id: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let primary = primary_url.trim();
    if !primary.is_empty() {
        urls.push(primary.to_string());
    }

    if let Some(archive_name) = official_archive_name_for_model_id(model_id) {
        let bos = format!(
            "https://paddle-model-ecology.bj.bcebos.com/paddlex/official_inference_model/paddle3.0.0/{}",
            archive_name
        );
        if !urls.contains(&bos) {
            urls.push(bos);
        }
    }

    urls
}

pub struct ModelManager {
    models_dir: PathBuf,
    cancellation_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
    network_monitor: Arc<PeerNetworkMonitor>,
}

impl ModelManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().ok_or(ModelError::NoConfigDir)?;
        let models_dir = config_dir
            .join(APP_DIR_NAME)
            .join("Local Storage")
            .join("models");

        fs::create_dir_all(&models_dir)?;

        let network_monitor = Arc::new(PeerNetworkMonitor::new());

        Ok(Self {
            models_dir,
            cancellation_tokens: Arc::new(Mutex::new(HashMap::new())),
            network_monitor,
        })
    }

    pub fn start_monitor(&self) {
        self.network_monitor.start_monitor();
    }

    pub fn models_dir(&self) -> &Path {
        &self.models_dir
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
        let dir = self.get_model_dir(model_id);
        dir.exists() && is_model_dir_ready(&dir)
    }

    pub fn list_downloaded_models(&self) -> Result<Vec<String>> {
        let mut models = Vec::new();

        if self.models_dir.exists() {
            for entry in fs::read_dir(&self.models_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() && is_model_dir_ready(&path) {
                    if let Some(name) = path.file_name() {
                        models.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(models)
    }

    pub fn cancel_download(&self, model_id: &str) {
        let canonical_id = canonical_ocr_model_id(model_id).to_string();
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            if let Some(token) = tokens.remove(&canonical_id) {
                token.cancel();
            }
        }
    }

    pub async fn download_and_extract<F>(
        &self,
        url: &str,
        model_id: &str,
        mut on_progress: F,
    ) -> Result<PathBuf>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        let canonical_id = canonical_ocr_model_id(model_id).to_string();
        let target_dir = self.get_model_dir(&canonical_id);

        if self.is_model_installed(&canonical_id) {
            return Ok(target_dir);
        }

        self.cancel_download(&canonical_id);

        let cancel_token = CancellationToken::new();
        if let Ok(mut tokens) = self.cancellation_tokens.lock() {
            tokens.insert(canonical_id.clone(), cancel_token.clone());
        }

        let temp_file_path = self.get_temp_file_path(&canonical_id);
        let client = reqwest::Client::builder()
            .user_agent("squigit-ocr-model-downloader/1.0")
            .build()?;
        let archive_candidates = build_archive_candidates(url, &canonical_id);
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

                let status_str = if current_bytes == 0 {
                    "checking"
                } else if net_state.status == NetworkStatus::Offline {
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
                        println!(
                            "Download attempt failed ({}, try {}/3): {}",
                            candidate_url, attempts, e
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
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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

            println!("Extracting model {}...", canonical_id);
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

        println!(
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

    async fn download_file_to_path(
        &self,
        client: &reqwest::Client,
        url: &str,
        target_path: &Path,
        cancel_token: &CancellationToken,
    ) -> Result<u64> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(ModelError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        let mut file = File::create(target_path).await?;
        let mut stream = response.bytes_stream();
        let mut written = 0u64;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return Err(ModelError::Cancelled);
                }
                item = stream.next() => {
                    match item {
                        Some(chunk_result) => {
                            let chunk = chunk_result?;
                            file.write_all(&chunk).await?;
                            written += chunk.len() as u64;
                        }
                        None => break,
                    }
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
        let canonical_id = canonical_ocr_model_id(model_id);
        let repo = hf_repo_for_model_id(canonical_id).ok_or_else(|| {
            ModelError::Extraction(format!(
                "No direct-download fallback mapping for model: {}",
                canonical_id
            ))
        })?;

        const HF_BASES: [&str; 2] = ["https://huggingface.co", "https://hf-mirror.com"];
        const REQUIRED_FILES: [&str; 3] =
            ["inference.json", "inference.pdiparams", "inference.yml"];
        let mut errors: Vec<String> = Vec::new();

        for base in HF_BASES {
            if cancel_token.is_cancelled() {
                return Err(ModelError::Cancelled);
            }

            if target_dir.exists() {
                fs::remove_dir_all(target_dir)?;
            }
            fs::create_dir_all(target_dir)?;

            let mut success = true;
            for (idx, file_name) in REQUIRED_FILES.iter().enumerate() {
                if cancel_token.is_cancelled() {
                    return Err(ModelError::Cancelled);
                }

                let progress = ((idx as f64 / REQUIRED_FILES.len() as f64) * 100.0) as u8;
                on_progress(DownloadProgressPayload {
                    id: model_id.to_string(),
                    progress,
                    loaded: idx as u64,
                    total: REQUIRED_FILES.len() as u64,
                    status: "downloading".to_string(),
                });

                let file_url = format!("{}/{}/resolve/main/{}", base, repo, file_name);
                let target_path = target_dir.join(file_name);
                match self
                    .download_file_to_path(client, &file_url, &target_path, cancel_token)
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        errors.push(format!("{}: {}", file_url, err));
                        success = false;
                        break;
                    }
                }
            }

            if success && is_model_dir_ready(target_dir) {
                on_progress(DownloadProgressPayload {
                    id: model_id.to_string(),
                    progress: 100,
                    loaded: REQUIRED_FILES.len() as u64,
                    total: REQUIRED_FILES.len() as u64,
                    status: "extracting".to_string(),
                });
                println!(
                    "Model {} installed successfully at {:?} (direct file fallback)",
                    model_id, target_dir
                );
                return Ok(target_dir.to_path_buf());
            }
        }

        Err(ModelError::Extraction(format!(
            "All fallback mirrors failed for {}:\n{}",
            model_id,
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

        let (mut total_size, can_resume) = match client.head(url).send().await {
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
                println!(
                    "HEAD {} returned {}; continuing without resume metadata",
                    url,
                    head_resp.status()
                );
                (0, false)
            }
            Err(err) => {
                println!("HEAD {} failed ({}); continuing with direct GET", url, err);
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

        let mut response = if downloaded_bytes > 0 {
            client
                .get(url)
                .header(RANGE, format!("bytes={}-", downloaded_bytes))
                .send()
                .await?
        } else {
            client.get(url).send().await?
        };

        if downloaded_bytes > 0 && response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            if total_size > 0 && downloaded_bytes >= total_size {
                return Ok(());
            }
            drop(file);
            let _ = fs::remove_file(temp_file_path);
            downloaded_bytes = 0;
            file = File::create(temp_file_path).await?;
            response = client.get(url).send().await?;
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

                            if total_size > 0 {
                                let progress = ((downloaded_bytes as f64 / total_size as f64) * 100.0) as u8;
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

        if target_dir.exists() {
            fs::remove_dir_all(target_dir)?;
        }
        fs::create_dir_all(target_dir)?;

        let tar_file = fs::File::open(temp_file_path)?;
        let mut archive = Archive::new(tar_file);

        if url.ends_with(".tar") {
            self.extract_archive(&mut archive, target_dir)?;
        } else if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            let tar_file = fs::File::open(temp_file_path)?;
            let tar = GzDecoder::new(tar_file);
            let mut archive = Archive::new(tar);
            self.extract_archive(&mut archive, target_dir)?;
        } else if self.extract_archive(&mut archive, target_dir).is_err() {
            if target_dir.exists() {
                fs::remove_dir_all(target_dir)?;
                fs::create_dir_all(target_dir)?;
            }
            let tar_file = fs::File::open(temp_file_path)?;
            let tar_gz = GzDecoder::new(tar_file);
            let mut archive_gz = Archive::new(tar_gz);
            self.extract_archive(&mut archive_gz, target_dir)
                .map_err(|e| ModelError::Extraction(format!("Failed to extract: {:?}", e)))?;
        }

        fs::remove_file(temp_file_path)?;
        println!(
            "Model {} installed successfully at {:?}",
            model_id, target_dir
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

#[cfg(test)]
mod tests {
    use super::{build_archive_candidates, canonical_ocr_model_id};

    #[test]
    fn canonical_model_id_defaults_to_english() {
        assert_eq!(canonical_ocr_model_id(""), "pp-ocr-v5-en");
        assert_eq!(canonical_ocr_model_id("   "), "pp-ocr-v5-en");
    }

    #[test]
    fn archive_candidates_keep_primary_and_official_fallback() {
        let candidates =
            build_archive_candidates("https://example.invalid/custom.tar", "pp-ocr-v5-cyrillic");
        assert_eq!(
            candidates.first().map(String::as_str),
            Some("https://example.invalid/custom.tar")
        );
        assert!(candidates
            .iter()
            .any(|candidate| candidate.contains("cyrillic_PP-OCRv5_mobile_rec_infer.tar")));
    }
}
