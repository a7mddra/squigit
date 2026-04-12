// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::{atomic::AtomicBool, Arc};

use ops_profile_store::auth::{start_google_auth_flow, validate_google_credentials, AuthFlowSettings};
use ops_profile_store::security::{encrypt_and_save_key, get_decrypted_key, ApiKeyProvider};
use ops_profile_store::{ProfileError, ProfileStore};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("usage: cargo run -p ops-profile-store --example live_store_harness -- <command>".to_string());
    };

    match command.as_str() {
        "store-base-dir" => {
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            println!("{}", store.base_dir().display());
        }
        "active-profile-id" => {
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            if let Some(profile_id) = store.get_active_profile_id().map_err(|err| err.to_string())? {
                println!("{}", profile_id);
            }
        }
        "profile-id-for-email" => {
            let email = args.next().ok_or_else(|| "missing email".to_string())?;
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            let profile = store
                .find_profile_by_email(&email)
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("Profile not found for email: {}", email))?;
            println!("{}", profile.id);
        }
        "set-active-profile" => {
            let profile_id = args.next().ok_or_else(|| "missing profile_id".to_string())?;
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            store
                .set_active_profile_id(&profile_id)
                .map_err(|err| err.to_string())?;
            println!("{}", profile_id);
        }
        "auth-google" => {
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            let settings = AuthFlowSettings::new(
                "Squigit",
                Arc::new(|url| open_url(url).map_err(ProfileError::Auth)),
            );
            let result = start_google_auth_flow(&store, &settings, Arc::new(AtomicBool::new(false)))
                .map_err(|err| err.to_string())?;
            println!("{}", serde_json::to_string(&result).map_err(|err| err.to_string())?);
        }
        "auth-credentials-ok" => {
            let settings = AuthFlowSettings::new(
                "Squigit",
                Arc::new(|_| Ok(())),
            );
            validate_google_credentials(&settings).map_err(|err| err.to_string())?;
            println!("ok");
        }
        "save-key" => {
            let profile_id = args.next().ok_or_else(|| "missing profile_id".to_string())?;
            let provider = args.next().ok_or_else(|| "missing provider".to_string())?;
            let plaintext = args.next().ok_or_else(|| "missing plaintext".to_string())?;
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            let provider = ApiKeyProvider::from_str(&provider).map_err(|err| err.to_string())?;
            let path =
                encrypt_and_save_key(&store, &profile_id, provider, &plaintext).map_err(|err| err.to_string())?;
            println!("{}", path);
        }
        "get-key" => {
            let profile_id = args.next().ok_or_else(|| "missing profile_id".to_string())?;
            let provider = args.next().ok_or_else(|| "missing provider".to_string())?;
            let store = ProfileStore::new().map_err(|err| err.to_string())?;
            let provider = ApiKeyProvider::from_str(&provider).map_err(|err| err.to_string())?;
            if let Some(value) = get_decrypted_key(&store, provider, &profile_id).map_err(|err| err.to_string())? {
                println!("{}", value);
            }
        }
        other => {
            return Err(format!("unknown command: {}", other));
        }
    }

    Ok(())
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let command = ("xdg-open", vec![url.to_string()]);
    #[cfg(target_os = "macos")]
    let command = ("open", vec![url.to_string()]);
    #[cfg(target_os = "windows")]
    let command = (
        "rundll32",
        vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
    );

    let output = Command::new(command.0)
        .args(&command.1)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|err| format!("Failed to open browser: {}", err))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!(
                "Browser opener exited with status {}",
                output.status
            ))
        } else {
            Err(format!(
                "Browser opener exited with status {}: {}",
                output.status, stderr
            ))
        }
    }
}
