// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    embed_google_credentials();
    tauri_build::build()
}

fn embed_google_credentials() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    let embedded_path = out_dir.join("snapllm-google-credentials.json");

    let payload = load_google_credentials_json().unwrap_or_default();
    if let Err(err) = fs::write(&embedded_path, payload) {
        panic!(
            "Failed to write embedded Google credentials at {}: {}",
            embedded_path.display(),
            err
        );
    }

    println!(
        "cargo:rustc-env=SNAPLLM_GOOGLE_CREDENTIALS_EMBEDDED_FILE={}",
        embedded_path.display()
    );
    println!("cargo:rerun-if-env-changed=SNAPLLM_GOOGLE_CREDENTIALS_JSON");
    println!("cargo:rerun-if-env-changed=SNAPLLM_GOOGLE_CREDENTIALS_PATH");

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let default_credentials_path = Path::new(&manifest_dir)
            .join("src")
            .join("data")
            .join("credentials.json");
        println!(
            "cargo:rerun-if-changed={}",
            default_credentials_path.display()
        );
    }
}

fn load_google_credentials_json() -> Option<String> {
    if let Ok(raw_json) = env::var("SNAPLLM_GOOGLE_CREDENTIALS_JSON") {
        let trimmed = raw_json.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(path) = env::var("SNAPLLM_GOOGLE_CREDENTIALS_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            match fs::read_to_string(trimmed) {
                Ok(contents) => return Some(contents),
                Err(err) => {
                    println!(
                        "cargo:warning=Failed to read SNAPLLM_GOOGLE_CREDENTIALS_PATH ({}): {}",
                        trimmed, err
                    );
                }
            }
        }
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let default_path = manifest_dir.join("src").join("data").join("credentials.json");
    fs::read_to_string(default_path).ok()
}
