#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod vault;
#[cfg(target_os = "windows")]
mod windows;

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use fs2::FileExt;
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

use crate::storage::AppData;

static REVEAL: Mutex<Option<RevealGrant>> = Mutex::new(None);
const REVEAL_GRACE_PERIOD: Duration = Duration::from_secs(600);
const KEY_DOMAIN: &[u8] = b"squigit/image-hosting/v1/imgbb/key";
const AAD: &[u8] = b"squigit/image-hosting/v1/imgbb/aes-256-gcm/hkdf-sha256";

struct RevealGrant {
    auth_path: PathBuf,
    verified_at: Instant,
}

pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn expose(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthFile {
    imgbb: Option<Record>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    cipher: Cipher,
    kdf: Kdf,
    salt: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Deserialize, Serialize)]
enum Cipher {
    #[serde(rename = "aes-256-gcm")]
    Aes256Gcm,
}
#[derive(Deserialize, Serialize)]
enum Kdf {
    #[serde(rename = "hkdf-sha256")]
    HkdfSha256,
}

pub fn valid_key(value: &str) -> bool {
    let value = value.trim();
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn access<T>(data: &AppData, action: impl FnOnce(&mut AuthFile) -> Result<T>) -> Result<T> {
    data.ensure()?;
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let lock = options.open(data.auth_path().with_file_name(".auth.lock"))?;
    lock.lock_exclusive()?;
    let mut auth = match fs::read(data.auth_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .context("Could not read the encrypted image-hosting keys")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => AuthFile::default(),
        Err(error) => return Err(error).context("Could not open the encrypted image-hosting keys"),
    };
    action(&mut auth)
}

fn persist(data: &AppData, auth: &AuthFile) -> Result<()> {
    let path = data.auth_path();
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing app data directory")?)?;
    file.write_all(&serde_json::to_vec_pretty(auth)?)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|error| error.error)
        .context("Could not save the encrypted image-hosting keys")?;
    Ok(())
}

fn derive(master: &[u8; 32], salt: &[u8]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0; 32]);
    Hkdf::<Sha256>::new(Some(salt), master)
        .expand(KEY_DOMAIN, &mut *key)
        .map_err(|_| anyhow::anyhow!("Could not derive the image-hosting encryption key"))?;
    Ok(key)
}

fn decode(value: &str, length: Option<usize>) -> Result<Vec<u8>> {
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .context("Invalid encrypted image-hosting record")?;
    anyhow::ensure!(
        length.is_none_or(|length| decoded.len() == length)
            && URL_SAFE_NO_PAD.encode(&decoded) == value,
        "Invalid encrypted image-hosting record"
    );
    Ok(decoded)
}

pub fn configured(data: &AppData) -> Result<bool> {
    access(data, |auth| Ok(auth.imgbb.is_some()))
}

pub fn save(data: &AppData, secret: &SecretString) -> Result<()> {
    let plaintext = secret.expose().trim();
    anyhow::ensure!(
        valid_key(plaintext),
        "Expected a 32-character ImgBB API key"
    );
    access(data, |auth| {
        let master = match vault::get()? {
            Some(master) => master,
            None if auth.imgbb.is_none() => vault::create()?,
            None => anyhow::bail!("The encryption master is missing from the OS vault. Your saved key was not changed."),
        };
        let mut salt = [0; 32];
        let mut nonce = [0; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce);
        let key = derive(&master, &salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..])
            .map_err(|_| anyhow::anyhow!("Could not initialize image-hosting encryption"))?;
        let ciphertext = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: AAD,
                },
            )
            .map_err(|_| anyhow::anyhow!("Could not encrypt the image-hosting key"))?;
        auth.imgbb = Some(Record {
            cipher: Cipher::Aes256Gcm,
            kdf: Kdf::HkdfSha256,
            salt: URL_SAFE_NO_PAD.encode(salt),
            nonce: URL_SAFE_NO_PAD.encode(nonce),
            ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
        });
        persist(data, auth)
    })
}

pub fn delete(data: &AppData) -> Result<()> {
    access(data, |auth| {
        auth.imgbb = None;
        persist(data, auth)
    })
}

pub fn load(data: &AppData) -> Result<Option<SecretString>> {
    access(data, |auth| {
        let Some(record) = &auth.imgbb else {
            return Ok(None);
        };
        let master = vault::get()?.context("The encryption master is missing from the OS vault")?;
        let salt = decode(&record.salt, Some(32))?;
        let nonce: [u8; 12] = decode(&record.nonce, Some(12))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid encrypted nonce"))?;
        let ciphertext = decode(&record.ciphertext, None)?;
        anyhow::ensure!(
            ciphertext.len() >= 16,
            "Invalid encrypted authentication tag"
        );
        let key = derive(&master, &salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..])
            .map_err(|_| anyhow::anyhow!("Could not initialize image-hosting encryption"))?;
        let plaintext = cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &ciphertext,
                    aad: AAD,
                },
            )
            .map_err(|_| {
                anyhow::anyhow!("The encrypted image-hosting key failed authentication")
            })?;
        let value = String::from_utf8(plaintext).map_err(|error| {
            let mut bytes = error.into_bytes();
            bytes.zeroize();
            anyhow::anyhow!("The encrypted image-hosting key is invalid")
        })?;
        let secret = SecretString::new(value);
        anyhow::ensure!(
            valid_key(secret.expose()),
            "The encrypted image-hosting key is invalid"
        );
        Ok(Some(secret))
    })
}

pub fn reveal(data: &AppData, window: Option<rune::WindowHandle>) -> Result<Option<SecretString>> {
    let mut grant = REVEAL
        .try_lock()
        .map_err(|_| anyhow::anyhow!("API key authentication is already in progress"))?;
    if !configured(data)? {
        *grant = None;
        return Ok(None);
    }
    if grant.as_ref().is_some_and(|grant| {
        grant.auth_path == data.auth_path() && grant.verified_at.elapsed() < REVEAL_GRACE_PERIOD
    }) {
        return load(data);
    }
    *grant = None;
    #[cfg(target_os = "linux")]
    let authorized = {
        let _ = window;
        linux::authenticate().map_err(anyhow::Error::msg)?
    };
    #[cfg(target_os = "macos")]
    let authorized = {
        let _ = window;
        macos::authenticate("reveal your ImgBB API key").map_err(anyhow::Error::msg)?
    };
    #[cfg(target_os = "windows")]
    let authorized = window
        .context("Open the configuration window before revealing your key")?
        .with_raw_handle(|handle| match handle.as_raw() {
            rune::raw_window_handle::RawWindowHandle::Win32(handle) => windows::authenticate(
                &(handle.hwnd.get() as usize).to_le_bytes(),
                "reveal your ImgBB API key",
            ),
            _ => Err("Windows Hello requires a native Squigit window".to_owned()),
        })
        .map_err(anyhow::Error::msg)?
        .map_err(anyhow::Error::msg)?;
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let authorized = {
        let _ = window;
        anyhow::bail!("Native authentication is unavailable on this operating system")
    };
    if !authorized {
        return Ok(None);
    }
    let verified_at = Instant::now();
    let secret = load(data)?;
    if secret.is_some() {
        *grant = Some(RevealGrant {
            auth_path: data.auth_path(),
            verified_at,
        });
    }
    Ok(secret)
}
