// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::services::ocr::DesktopOcrService;
use ops_squigit_brain::provider::attachments::resolve_attachment_path_buf;
use ops_squigit_ocr::ocr::{OcrBox, OcrRequest};
use tauri::Manager;

#[tauri::command]
pub async fn ocr_image(
    app: tauri::AppHandle,
    ocr: tauri::State<'_, DesktopOcrService>,
    image_data: String,
    is_base64: bool,
    model_name: Option<String>,
) -> Result<Vec<OcrBox>, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let (sidecar_path, runtime_dir) = ocr.resolve_sidecar_path(&resource_dir);

    if sidecar_path.is_absolute() && !sidecar_path.exists() {
        return Err("ERR_MISSING_OCR_PACKAGE".to_string());
    }

    ocr.ensure_sidecar_version_compatible(&sidecar_path)?;

    if is_base64 {
        return Err(
            "OCR sidecar is path-only. Pass a stored CAS path instead of base64 data.".to_string(),
        );
    }

    let resolved_image_path = resolve_attachment_path_buf(&image_data)?;

    let rec_model_dir_override = ocr.resolve_rec_model_dir_override(model_name.as_deref());

    let result = ocr
        .run_ocr(OcrRequest {
            sidecar_path,
            runtime_dir,
            image_path: resolved_image_path,
            rec_model_dir_override,
            timeout_secs: None,
        })
        .await?;

    Ok(result.boxes)
}

/// Cancel the currently running OCR job.
/// Kills the sidecar process and waits briefly for shutdown.
/// This is fire-and-forget from the frontend's perspective.
#[tauri::command]
pub async fn cancel_ocr_job(ocr: tauri::State<'_, DesktopOcrService>) -> Result<(), String> {
    ocr.cancel_ocr_job().await
}
