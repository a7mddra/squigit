// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_squigit_ocr::models::{DownloadProgressPayload, ModelError, ModelManager};
use ops_squigit_ocr::ocr::{OcrExecutionResult, OcrRequest, OcrRuntime, OcrRuntimeError};
use ops_squigit_ocr::sidecar::{
    DEFAULT_OCR_VERSION_REQUIREMENT, SidecarError, check_ocr_version_requirement,
    read_sidecar_version, resolve_sidecar_path,
};
use std::path::{Path, PathBuf};

pub struct DesktopOcrService {
    model_manager: ModelManager,
    runtime: OcrRuntime,
}

impl DesktopOcrService {
    pub fn new() -> Result<Self, ModelError> {
        Ok(Self {
            model_manager: ModelManager::new()?,
            runtime: OcrRuntime::new(),
        })
    }

    pub fn start_monitor(&self) {
        self.model_manager.start_monitor();
    }

    pub async fn download_model<F>(
        &self,
        url: &str,
        model_id: &str,
        on_progress: F,
    ) -> Result<PathBuf, String>
    where
        F: FnMut(DownloadProgressPayload) + Send,
    {
        self.model_manager
            .download_and_extract(url, model_id, on_progress)
            .await
            .map_err(|e| e.to_string())
    }

    pub fn cancel_model_download(&self, model_id: &str) {
        self.model_manager.cancel_download(model_id);
    }

    pub fn list_downloaded_models(&self) -> Result<Vec<String>, String> {
        self.model_manager
            .list_downloaded_models()
            .map_err(|e| e.to_string())
    }

    pub fn get_model_dir(&self, model_id: &str) -> PathBuf {
        self.model_manager.get_model_dir(model_id)
    }

    pub fn is_model_installed(&self, model_id: &str) -> bool {
        self.model_manager.is_model_installed(model_id)
    }

    pub fn resolve_rec_model_dir_override(&self, model_name: Option<&str>) -> Option<PathBuf> {
        model_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .and_then(|name| {
                if self.is_model_installed(name) {
                    Some(self.get_model_dir(name))
                } else {
                    None
                }
            })
    }

    pub fn resolve_sidecar_path(&self, resource_dir: &Path) -> (PathBuf, Option<PathBuf>) {
        resolve_sidecar_path(resource_dir)
    }

    pub fn ensure_sidecar_version_compatible(&self, sidecar_path: &Path) -> Result<(), String> {
        check_ocr_version_requirement(sidecar_path, DEFAULT_OCR_VERSION_REQUIREMENT)
            .map(|_| ())
            .map_err(map_sidecar_error)
    }

    pub fn read_sidecar_version(&self, sidecar_path: &Path) -> Result<String, String> {
        read_sidecar_version(sidecar_path).map_err(map_sidecar_error)
    }

    pub async fn run_ocr(&self, request: OcrRequest) -> Result<OcrExecutionResult, String> {
        self.runtime
            .run(request)
            .await
            .map_err(map_ocr_runtime_error)
    }

    pub async fn cancel_ocr_job(&self) -> Result<(), String> {
        self.runtime
            .cancel_current_job()
            .await
            .map_err(map_ocr_runtime_error)
    }
}

fn map_sidecar_error(error: SidecarError) -> String {
    match error {
        SidecarError::MissingPackage => "ERR_MISSING_OCR_PACKAGE".to_string(),
        SidecarError::OutdatedPackage => "ERR_OUTDATED_OCR_PACKAGE".to_string(),
        _ => error.to_string(),
    }
}

fn map_ocr_runtime_error(error: OcrRuntimeError) -> String {
    match error {
        OcrRuntimeError::MissingPackage => "ERR_MISSING_OCR_PACKAGE".to_string(),
        OcrRuntimeError::Cancelled => "OCR job was cancelled".to_string(),
        OcrRuntimeError::Message(message) => message,
    }
}
