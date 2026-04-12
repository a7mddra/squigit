// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2;
use rand::{rngs::OsRng, RngCore};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::str::FromStr;

use crate::{ProfileError, ProfileStore, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyProvider {
    GoogleAiStudio,
    ImgBb,
}

impl ApiKeyProvider {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::GoogleAiStudio => "Google AI Studio",
            Self::ImgBb => "ImgBB",
        }
    }

    pub fn storage_key_name(self) -> &'static str {
        match self {
            Self::GoogleAiStudio => "google ai studio",
            Self::ImgBb => "imgbb",
        }
    }

    pub fn is_valid_key(self, key: &str) -> bool {
        if key.is_empty() {
            return true;
        }

        match self {
            Self::GoogleAiStudio => key.starts_with("AIzaS") && key.len() == 39,
            Self::ImgBb => key.len() == 32,
        }
    }

    pub fn validation_hint(self) -> &'static str {
        match self {
            Self::GoogleAiStudio => "Expected a key that starts with 'AIzaS' and is 39 characters long.",
            Self::ImgBb => "Expected a 32-character API key.",
        }
    }
}

impl FromStr for ApiKeyProvider {
    type Err = ProfileError;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "google ai studio" | "google_ai_studio" | "google-ai-studio" | "gemini" => {
                Ok(Self::GoogleAiStudio)
            }
            "imgbb" => Ok(Self::ImgBb),
            other => Err(ProfileError::InvalidProvider(other.to_string())),
        }
    }
}

fn get_stable_passphrase() -> Result<String> {
    let home_dir = dirs::home_dir().ok_or(ProfileError::NoConfigDir)?;
    let home_str = home_dir.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(home_str.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    pbkdf2::<hmac::Hmac<Sha256>>(passphrase.as_bytes(), salt, 150_000, &mut key)
        .map_err(|err| ProfileError::Security(format!("PBKDF2 derivation failed: {}", err)))?;
    Ok(key)
}

pub fn get_decrypted_key(
    store: &ProfileStore,
    provider: ApiKeyProvider,
    profile_id: &str,
) -> Result<Option<String>> {
    let file_path = store.get_provider_key_path(profile_id, provider.storage_key_name());
    if !file_path.exists() {
        return Ok(None);
    }

    let file_content = fs::read_to_string(file_path)?;
    let payload: Value = serde_json::from_str(&file_content)?;

    let decode = |field: &str| -> Result<Vec<u8>> {
        let value = payload
            .get(field)
            .and_then(|value| value.as_str())
            .ok_or_else(|| ProfileError::Security(format!("Encrypted key payload missing '{}'", field)))?;
        general_purpose::STANDARD
            .decode(value)
            .map_err(|err| ProfileError::Security(format!("Invalid base64 for '{}': {}", field, err)))
    };

    let salt = decode("salt")?;
    let iv = decode("iv")?;
    let tag = decode("tag")?;
    let ciphertext = decode("ciphertext")?;

    let passphrase = get_stable_passphrase()?;
    let key_bytes = derive_key(&passphrase, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    let mut encrypted_data = ciphertext;
    encrypted_data.extend_from_slice(&tag);

    let plaintext_bytes = cipher
        .decrypt(nonce, encrypted_data.as_ref())
        .map_err(|_| ProfileError::Security("Failed to decrypt stored API key".to_string()))?;

    let plaintext = String::from_utf8(plaintext_bytes)
        .map_err(|err| ProfileError::Security(format!("Stored API key is not valid UTF-8: {}", err)))?;

    Ok(Some(plaintext))
}

pub fn encrypt_and_save_key(
    store: &ProfileStore,
    profile_id: &str,
    provider: ApiKeyProvider,
    plaintext: &str,
) -> Result<String> {
    if store.get_profile(profile_id)?.is_none() {
        return Err(ProfileError::ProfileNotFound(profile_id.to_string()));
    }

    let plaintext = plaintext.trim();
    validate_api_key(provider, plaintext)?;

    let passphrase = get_stable_passphrase()?;
    let mut salt = [0u8; 16];
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut iv);

    let key_bytes = derive_key(&passphrase, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    let encrypted_data = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|err| ProfileError::Security(format!("Encryption failed: {}", err)))?;

    let (ciphertext, tag) = encrypted_data.split_at(encrypted_data.len() - 16);
    let payload = serde_json::json!({
        "version": 1,
        "algo": "aes-256-gcm",
        "salt": general_purpose::STANDARD.encode(salt),
        "iv": general_purpose::STANDARD.encode(iv),
        "tag": general_purpose::STANDARD.encode(tag),
        "ciphertext": general_purpose::STANDARD.encode(ciphertext)
    });

    let file_path = store.get_provider_key_path(profile_id, provider.storage_key_name());
    store.write_json_atomic(&file_path, &payload)?;

    Ok(file_path.to_string_lossy().to_string())
}

pub fn validate_api_key(provider: ApiKeyProvider, plaintext: &str) -> Result<()> {
    let trimmed = plaintext.trim();
    if provider.is_valid_key(trimmed) {
        return Ok(());
    }

    Err(ProfileError::Security(format!(
        "Invalid {} API key format. {}",
        provider.display_name(),
        provider.validation_hint()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Profile;
    use tempfile::tempdir;

    fn temp_store() -> ProfileStore {
        let temp_dir = tempdir().unwrap();
        let root = temp_dir.path().to_path_buf();
        std::mem::forget(temp_dir);
        let store = ProfileStore::with_base_dir(root.join("Local Storage")).unwrap();
        let profile = Profile::new("auth@example.com", "Auth User", None, None);
        store.upsert_profile(&profile).unwrap();
        store
    }

    fn valid_google_key() -> String {
        format!("AIzaS{}", "1".repeat(34))
    }

    #[test]
    fn round_trip_encrypted_api_key() {
        let store = temp_store();
        let profile_id = store
            .list_profiles()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .id;

        encrypt_and_save_key(
            &store,
            &profile_id,
            ApiKeyProvider::GoogleAiStudio,
            &valid_google_key(),
        )
        .unwrap();

        let value = get_decrypted_key(&store, ApiKeyProvider::GoogleAiStudio, &profile_id)
            .unwrap()
            .unwrap();
        assert_eq!(value, valid_google_key());
    }

    #[test]
    fn invalid_google_key_is_rejected() {
        let store = temp_store();
        let profile_id = store
            .list_profiles()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .id;

        let err = encrypt_and_save_key(
            &store,
            &profile_id,
            ApiKeyProvider::GoogleAiStudio,
            "not-a-valid-key",
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "Invalid Google AI Studio API key format. Expected a key that starts with 'AIzaS' and is 39 characters long."
        );
    }
}
