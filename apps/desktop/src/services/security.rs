// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::utils::get_app_config_dir;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2;
use sha2::{Digest, Sha256};
use std::fs;
use tauri::AppHandle;

pub fn get_stable_passphrase() -> String {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let home_str = home_dir.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(home_str.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::<hmac::Hmac<Sha256>>(passphrase.as_bytes(), salt, 150_000, &mut key)
        .expect("PBKDF2 derivation failed");
    key
}

pub fn get_decrypted_key_internal(
    app: &AppHandle,
    provider: &str,
    profile_id: &str,
) -> Option<String> {
    let config_dir = get_app_config_dir(app);
    let file_path = config_dir
        .join("Local Storage")
        .join(profile_id)
        .join(format!("{}_key.json", provider));

    if !file_path.exists() {
        return None;
    }

    let file_content = fs::read_to_string(file_path).ok()?;
    let payload: serde_json::Value = serde_json::from_str(&file_content).ok()?;

    let salt = general_purpose::STANDARD
        .decode(payload["salt"].as_str()?)
        .ok()?;
    let iv = general_purpose::STANDARD
        .decode(payload["iv"].as_str()?)
        .ok()?;
    let tag = general_purpose::STANDARD
        .decode(payload["tag"].as_str()?)
        .ok()?;
    let ciphertext = general_purpose::STANDARD
        .decode(payload["ciphertext"].as_str()?)
        .ok()?;

    let passphrase = get_stable_passphrase();
    let key_bytes = derive_key(&passphrase, &salt);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    let mut encrypted_data = ciphertext;
    encrypted_data.extend_from_slice(&tag);

    let plaintext_bytes = cipher.decrypt(nonce, encrypted_data.as_ref()).ok()?;

    String::from_utf8(plaintext_bytes).ok()
}

pub fn encrypt_and_save_key(
    app: &AppHandle,
    plaintext: &str,
    provider: &str,
    profile_id: &str,
) -> Result<String, String> {
    use rand::{rngs::OsRng, RngCore};
    use std::fs::File;
    use std::io::Write;

    let passphrase = get_stable_passphrase();
    let mut salt = [0u8; 16];
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut iv);

    let key_bytes = derive_key(&passphrase, &salt);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    let encrypted_data = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let (ciphertext, tag) = encrypted_data.split_at(encrypted_data.len() - 16);

    let payload = serde_json::json!({
        "version": 1,
        "algo": "aes-256-gcm",
        "salt": general_purpose::STANDARD.encode(salt),
        "iv": general_purpose::STANDARD.encode(iv),
        "tag": general_purpose::STANDARD.encode(tag),
        "ciphertext": general_purpose::STANDARD.encode(ciphertext)
    });

    // Store in profile-specific directory: Local Storage/{profile_id}/{provider}_key.json
    let profile_dir = get_app_config_dir(app)
        .join("Local Storage")
        .join(profile_id);
    fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;

    let file_path = profile_dir.join(format!("{}_key.json", provider));
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;

    file.write_all(serde_json::to_string_pretty(&payload).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}
