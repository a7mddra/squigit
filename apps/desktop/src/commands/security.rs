// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use ops_profile_store::security::ApiKeyProvider;
use ops_profile_store::ProfileStore;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn check_file_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
pub async fn encrypt_and_save(
    _app: AppHandle,
    profile_id: String,
    provider: String,
    plaintext: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = ProfileStore::new().map_err(|err| err.to_string())?;
        let provider = ApiKeyProvider::from_str(&provider).map_err(|err| err.to_string())?;
        ops_profile_store::security::encrypt_and_save_key(&store, &profile_id, provider, &plaintext)
            .map(|_| ())
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_agreed_flag(app: AppHandle) -> Result<(), String> {
    let config_dir = match app.path().app_config_dir() {
        Ok(path) => path,
        Err(e) => return Err(format!("Could not resolve app config dir: {}", e)),
    };

    if !config_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            return Err(format!("Failed to create config dir: {}", e));
        }
    }
    let marker_file = config_dir.join(".agreed");
    if let Err(e) = std::fs::write(&marker_file, "") {
        log::error!("Failed to create .agreed marker file: {}", e);
        Err(format!("Failed to create .agreed marker file: {}", e))
    } else {
        log::info!("Successfully created .agreed marker file");
        Ok(())
    }
}

#[tauri::command]
pub fn has_agreed_flag(app: AppHandle) -> bool {
    let config_dir = match app.path().app_config_dir() {
        Ok(path) => path,
        Err(e) => {
            log::error!("Could not resolve app config dir: {}", e);
            return false;
        }
    };
    config_dir.join(".agreed").exists()
}
