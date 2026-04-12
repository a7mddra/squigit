// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use ops_profile_store::auth::{self, AuthFlowSettings};
use ops_profile_store::security::ApiKeyProvider;
use ops_profile_store::{ProfileError, ProfileStore};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

const MISSING_CREDENTIALS_PREFIX: &str = "Google authentication is not configured in this build.";

#[derive(Debug, Clone, Serialize)]
struct AuthFailureData {
    error: String,
    cancelled: bool,
}

fn build_auth_settings() -> AuthFlowSettings {
    AuthFlowSettings::new(
        crate::constants::APP_NAME,
        Arc::new(|url| crate::utils::open_url(url).map_err(ProfileError::Auth)),
    )
}

fn log_auth_error(error: &str) {
    if error.starts_with(MISSING_CREDENTIALS_PREFIX) {
        // Missing credentials are already logged in ops-profile-store with
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
    let app_for_success = app.clone();

    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let store = ProfileStore::new().map_err(|err| err.to_string())?;
        let settings = build_auth_settings();
        let user_data =
            auth::start_google_auth_flow(&store, &settings, auth_cancelled).map_err(|err| err.to_string())?;
        app_for_success.emit("auth-success", &user_data)
            .map_err(|err| err.to_string())?;
        Ok(())
    })
    .await;

    auth_lock.store(false, Ordering::SeqCst);

    let result = result.map_err(|e| e.to_string())?;
    if let Err(ref e) = result {
        log_auth_error(e);
        let is_cancelled = e.contains("cancelled") || e.contains("expired");
        let payload = AuthFailureData {
            error: e.clone(),
            cancelled: is_cancelled,
        };
        if let Err(emit_err) = app.emit("auth-failure", &payload) {
            eprintln!("[auth] failed to emit auth-failure: {}", emit_err);
        }
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
    let cancel_url = build_auth_settings().cancel_url();

    // Trigger the cancellation by sending a request to the local server
    tauri::async_runtime::spawn_blocking(move || {
        let client = reqwest::blocking::Client::new();
        // Fire and forget - if it fails, the server might already be down
        let _ = client.get(cancel_url).send();
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
        if let Ok(store) = ProfileStore::new() {
            let _ = store.clear_active_profile_id();
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_api_key(
    provider: String,
    profile_id: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = ProfileStore::new().map_err(|err| err.to_string())?;
        let provider = ApiKeyProvider::from_str(&provider).map_err(|err| err.to_string())?;
        let key = ops_profile_store::security::get_decrypted_key(&store, provider, &profile_id)
            .map_err(|err| err.to_string())?;
        Ok::<String, String>(key.unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
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
        let store = ProfileStore::new().map_err(|err| err.to_string())?;
        auth::cache_avatar(&store, &url, profile_id.as_deref()).map_err(|err| err.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
