// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::constants::*;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConstants {
    pub app_name: &'static str,
    pub default_model: &'static str,
    pub default_theme: &'static str,
    pub default_prompt: &'static str,
    pub preferences_file_name: &'static str,
    pub default_capture_type: &'static str,
    pub default_ocr_language: &'static str,
    pub default_active_account: &'static str,
}

#[tauri::command]
pub fn get_app_constants() -> AppConstants {
    AppConstants {
        app_name: APP_NAME,
        default_model: DEFAULT_MODEL,
        default_theme: DEFAULT_THEME,
        default_prompt: DEFAULT_PROMPT,
        preferences_file_name: PREFERENCES_FILE_NAME,
        default_capture_type: DEFAULT_CAPTURE_TYPE,
        default_ocr_language: DEFAULT_OCR_LANGUAGE,
        default_active_account: DEFAULT_ACTIVE_ACCOUNT,
    }
}
