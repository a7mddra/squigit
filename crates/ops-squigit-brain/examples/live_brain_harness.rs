// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_profile_store::security::{get_decrypted_key, ApiKeyProvider};
use ops_profile_store::ProfileStore;
use ops_squigit_brain::constants::DEFAULT_MODEL;
use ops_squigit_brain::events::NoopEventSink;
use ops_squigit_brain::service::{
    AnalyzeImageRequest, BrainService, PromptChatRequest,
};
use serde_json::json;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(
            "usage: cargo run -p ops-squigit-brain --example live_brain_harness -- <analyze|prompt> ..."
                .to_string(),
        );
    };

    match command.as_str() {
        "analyze" => {
            let image_path = args.next().ok_or_else(|| "missing image_path".to_string())?;
            let user_message = args.next();
            let api_key = active_google_api_key()?;
            let service = BrainService::new();
            let sink = NoopEventSink;
            let result = service
                .analyze_image(
                    &sink,
                    AnalyzeImageRequest {
                        api_key,
                        model: DEFAULT_MODEL.to_string(),
                        image_path,
                        user_message,
                        channel_id: format!("cli-analyze-{}", chrono::Utc::now().timestamp_millis()),
                        user_name: None,
                        user_email: None,
                        user_instruction: None,
                        ocr_lang: None,
                    },
                )
                .await?;

            println!(
                "{}",
                serde_json::to_string(&json!({
                    "chat_id": result.metadata.id,
                    "title": result.metadata.title,
                    "assistant_message": result.assistant_message,
                    "image_path": result.image.path,
                }))
                .map_err(|e| e.to_string())?
            );
        }
        "prompt" => {
            let chat_id = args.next().ok_or_else(|| "missing chat_id".to_string())?;
            let message = args.collect::<Vec<_>>().join(" ");
            if message.trim().is_empty() {
                return Err("missing message".to_string());
            }

            let api_key = active_google_api_key()?;
            let service = BrainService::new();
            let sink = NoopEventSink;
            let result = service
                .prompt_chat(
                    &sink,
                    PromptChatRequest {
                        api_key,
                        model: DEFAULT_MODEL.to_string(),
                        chat_id,
                        user_message: message,
                        channel_id: format!("cli-prompt-{}", chrono::Utc::now().timestamp_millis()),
                        user_name: None,
                        user_email: None,
                    },
                )
                .await?;

            println!(
                "{}",
                serde_json::to_string(&json!({
                    "chat_id": result.chat_id,
                    "assistant_message": result.assistant_message,
                }))
                .map_err(|e| e.to_string())?
            );
        }
        other => return Err(format!("unknown command: {}", other)),
    }

    Ok(())
}

fn active_google_api_key() -> Result<String, String> {
    let store = ProfileStore::new().map_err(|e| e.to_string())?;
    let active_id = store
        .get_active_profile_id()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No active profile. Sign in first.".to_string())?;

    let provider = ApiKeyProvider::from_str("google ai studio").map_err(|e| e.to_string())?;
    let key = get_decrypted_key(&store, provider, &active_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if key.trim().is_empty() {
        return Err("Missing Google AI Studio API key for active profile.".to_string());
    }

    Ok(key)
}
