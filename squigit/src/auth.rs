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

use crate::image_hosting::Provider;
use crate::storage::AppData;

static REVEAL: Mutex<Option<RevealGrant>> = Mutex::new(None);
const REVEAL_GRACE_PERIOD: Duration = Duration::from_secs(600);

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
    imagekit: Option<Record>,
    chevereto: Option<Record>,
}

impl AuthFile {
    fn record(&self, provider: Provider) -> &Option<Record> {
        match provider {
            Provider::Imgbb => &self.imgbb,
            Provider::Imagekit => &self.imagekit,
            Provider::Chevereto => &self.chevereto,
        }
    }

    fn record_mut(&mut self, provider: Provider) -> &mut Option<Record> {
        match provider {
            Provider::Imgbb => &mut self.imgbb,
            Provider::Imagekit => &mut self.imagekit,
            Provider::Chevereto => &mut self.chevereto,
        }
    }

    fn is_empty(&self) -> bool {
        Provider::ALL
            .iter()
            .all(|provider| self.record(*provider).is_none())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    cipher: Cipher,
    kdf: Kdf,
    salt: String,
    nonce: String,
    ciphertext: String,
    endpoint: Option<String>,
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

fn derive(master: &[u8; 32], salt: &[u8], provider: Provider) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0; 32]);
    Hkdf::<Sha256>::new(Some(salt), master)
        .expand(
            format!("squigit/image-hosting/v1/{}/key", provider.id()).as_bytes(),
            &mut *key,
        )
        .map_err(|_| anyhow::anyhow!("Could not derive the image-hosting encryption key"))?;
    Ok(key)
}

fn aad(provider: Provider, endpoint: Option<&str>) -> String {
    let mut value = format!(
        "squigit/image-hosting/v1/{}/aes-256-gcm/hkdf-sha256",
        provider.id()
    );
    if let Some(endpoint) = endpoint {
        value.push('\0');
        value.push_str(endpoint);
    }
    value
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

pub fn configured(data: &AppData, provider: Provider) -> Result<bool> {
    access(data, |auth| Ok(auth.record(provider).is_some()))
}

pub fn status(data: &AppData) -> Result<[bool; 3]> {
    access(data, |auth| {
        Ok(Provider::ALL.map(|provider| auth.record(provider).is_some()))
    })
}

pub fn save(
    data: &AppData,
    provider: Provider,
    secret: &SecretString,
    endpoint: &str,
) -> Result<()> {
    let plaintext = secret.expose().trim();
    anyhow::ensure!(
        provider.valid_key(plaintext),
        "Enter a valid {} API key",
        provider.name()
    );
    let endpoint = if provider == Provider::Chevereto {
        Some(crate::image_hosting::custom_endpoint(endpoint)?.to_string())
    } else {
        None
    };
    access(data, |auth| {
        let master = match vault::get()? {
            Some(master) => master,
            None if auth.is_empty() => vault::create()?,
            None => anyhow::bail!("The encryption master is missing from the OS vault. Your saved key was not changed."),
        };
        let mut salt = [0; 32];
        let mut nonce = [0; 12];
        OsRng.fill_bytes(&mut salt);
        OsRng.fill_bytes(&mut nonce);
        let key = derive(&master, &salt, provider)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..])
            .map_err(|_| anyhow::anyhow!("Could not initialize image-hosting encryption"))?;
        let ciphertext = cipher
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: plaintext.as_bytes(),
                    aad: aad(provider, endpoint.as_deref()).as_bytes(),
                },
            )
            .map_err(|_| anyhow::anyhow!("Could not encrypt the image-hosting key"))?;
        *auth.record_mut(provider) = Some(Record {
            cipher: Cipher::Aes256Gcm,
            kdf: Kdf::HkdfSha256,
            salt: URL_SAFE_NO_PAD.encode(salt),
            nonce: URL_SAFE_NO_PAD.encode(nonce),
            ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
            endpoint,
        });
        persist(data, auth)
    })
}

pub fn delete(data: &AppData, provider: Provider) -> Result<()> {
    access(data, |auth| {
        *auth.record_mut(provider) = None;
        persist(data, auth)
    })
}

pub fn load(data: &AppData, provider: Provider, endpoint: &str) -> Result<Option<SecretString>> {
    let endpoint = if provider == Provider::Chevereto {
        Some(crate::image_hosting::custom_endpoint(endpoint)?.to_string())
    } else {
        None
    };
    access(data, |auth| {
        let Some(record) = auth.record(provider) else {
            return Ok(None);
        };
        anyhow::ensure!(
            record.endpoint == endpoint,
            "The image host changed. Save its API key again."
        );
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
        let key = derive(&master, &salt, provider)?;
        let cipher = Aes256Gcm::new_from_slice(&key[..])
            .map_err(|_| anyhow::anyhow!("Could not initialize image-hosting encryption"))?;
        let plaintext = cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &ciphertext,
                    aad: aad(provider, endpoint.as_deref()).as_bytes(),
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
            provider.valid_key(secret.expose()),
            "The encrypted image-hosting key is invalid"
        );
        Ok(Some(secret))
    })
}

pub fn reveal(
    data: &AppData,
    provider: Provider,
    window: Option<rune::WindowHandle>,
) -> Result<Option<SecretString>> {
    let mut grant = REVEAL
        .try_lock()
        .map_err(|_| anyhow::anyhow!("API key authentication is already in progress"))?;
    if !configured(data, provider)? {
        *grant = None;
        return Ok(None);
    }
    if grant.as_ref().is_some_and(|grant| {
        grant.auth_path == data.auth_path() && grant.verified_at.elapsed() < REVEAL_GRACE_PERIOD
    }) {
        return load(data, provider, &data.load()?.hosting.chevereto_url);
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
        macos::authenticate(&format!("reveal your {} API key", provider.name()))
            .map_err(anyhow::Error::msg)?
    };
    #[cfg(target_os = "windows")]
    let authorized = window
        .context("Open the configuration window before revealing your key")?
        .with_raw_handle(|handle| match handle.as_raw() {
            rune::raw_window_handle::RawWindowHandle::Win32(handle) => windows::authenticate(
                &(handle.hwnd.get() as usize).to_le_bytes(),
                &format!("reveal your {} API key", provider.name()),
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
    let secret = load(data, provider, &data.load()?.hosting.chevereto_url)?;
    if secret.is_some() {
        *grant = Some(RevealGrant {
            auth_path: data.auth_path(),
            verified_at,
        });
    }
    Ok(secret)
}
