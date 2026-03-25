// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::fs::{self, File};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

use crate::services::auth;
use crate::services::security;
use crate::state::AppState;
use crate::utils::get_app_config_dir;

const MISSING_CREDENTIALS_PREFIX: &str = "Google authentication is not configured in this build.";

fn log_auth_error(error: &str) {
    if error.starts_with(MISSING_CREDENTIALS_PREFIX) {
        // Missing credentials are already logged in services/auth.rs with
        // contributor setup instructions.
        return;
    }

    eprintln!(
        "[auth] start_google_auth failed: {}",
        error.replace('\n', "\n[auth] ")
    );
}

#[tauri::command]
pub async fn start_google_auth(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.auth_running.load(Ordering::SeqCst) {
        return Err("Authentication already in progress".into());
    }

    state.auth_running.store(true, Ordering::SeqCst);
    // Clear cancelled flag when starting fresh auth
    state.auth_cancelled.store(false, Ordering::SeqCst);
    let auth_lock = state.auth_running.clone();
    let auth_cancelled = state.auth_cancelled.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let config_dir = get_app_config_dir(&app);
        if !config_dir.exists() {
            match fs::create_dir_all(&config_dir) {
                Ok(_) => {}
                Err(e) => return Err(e.to_string()),
            }
        }

        auth::start_google_auth_flow(app, config_dir, auth_cancelled)
    })
    .await;

    auth_lock.store(false, Ordering::SeqCst);

    let result = result.map_err(|e| e.to_string())?;
    if let Err(ref e) = result {
        log_auth_error(e);
    }
    result
}

#[tauri::command]
pub async fn cancel_google_auth(_app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if !state.auth_running.load(Ordering::SeqCst) {
        return Ok(()); // Nothing to cancel
    }

    // Mark as cancelled so late callbacks are rejected
    state.auth_cancelled.store(true, Ordering::SeqCst);

    // Trigger the cancellation by sending a request to the local server
    tauri::async_runtime::spawn_blocking(|| {
        let client = reqwest::blocking::Client::new();
        // Fire and forget - if it fails, the server might already be down
        let _ = client
            .get(format!(
                "http://localhost:3000/{}-cancel",
                crate::constants::APP_NAME.to_lowercase()
            ))
            .send();
    })
    .await
    .map_err(|e| e.to_string())?;

    // We don't manually clear the flag here because the start_google_auth command
    // will clear it when the server loop breaks and the function returns.
    Ok(())
}

#[tauri::command]
pub async fn logout(_app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Clear active profile in index (Guest mode)
        // Profile data stays on disk for re-login
        if let Ok(store) = ops_profile_store::ProfileStore::new() {
            let _ = store.clear_active_profile_id();
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_user_data(app: AppHandle) -> serde_json::Value {
    let config_dir = get_app_config_dir(&app);
    let profile_path = config_dir.join("profile.json");

    if profile_path.exists() {
        if let Ok(file) = File::open(profile_path) {
            if let Ok(json) = serde_json::from_reader(file) {
                return json;
            }
        }
    }

    serde_json::json!({
        "name": "Guest User",
        "email": "Not logged in",
        "avatar": ""
    })
}

#[tauri::command]
pub async fn get_api_key(
    app: AppHandle,
    provider: String,
    profile_id: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        security::get_decrypted_key_internal(&app, &provider, &profile_id).unwrap_or_default()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Cache an avatar image from a remote URL to local CAS storage.
/// Returns the local file path on success.
#[tauri::command]
pub async fn cache_avatar(
    _app: AppHandle,
    url: String,
    profile_id: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Download image
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to download avatar: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download avatar: HTTP {}",
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read avatar bytes: {}", e))?;

        // Initialize profile store
        let profile_store = ops_profile_store::ProfileStore::new()
            .map_err(|e| format!("Failed to initialize profile store: {}", e))?;

        // Determine target profile ID: explicit > active > error
        let target_id = match profile_id {
            Some(id) => id,
            None => profile_store
                .get_active_profile_id()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "No active profile and no profile ID provided.".to_string())?,
        };

        // Get storage directory for the target profile
        let chats_dir = profile_store.get_chats_dir(&target_id);
        let storage = ops_chat_storage::ChatStorage::with_base_dir(chats_dir)
            .map_err(|e| format!("Failed to initialize storage: {}", e))?;

        // Store image
        let stored_image = storage
            .store_image(&bytes, None)
            .map_err(|e| format!("Failed to store avatar: {}", e))?;

        let local_path = stored_image.path.clone();

        // Update profile with new avatar path
        if let Some(mut profile) = profile_store.get_profile(&target_id).ok().flatten() {
            profile.avatar = Some(local_path.clone());
            let _ = profile_store.upsert_profile(&profile);
        }

        Ok(local_path)
    })
    .await
    .map_err(|e| e.to_string())?
}
