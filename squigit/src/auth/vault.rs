use anyhow::{Context, Result};
use keyring::v1::Entry;
use rand::{rngs::OsRng, RngCore};
use zeroize::{Zeroize, Zeroizing};

fn entry() -> Result<Entry> {
    Entry::new("io.github.a7mddra.squigit", "image-hosting-master-v1")
        .map_err(|_| anyhow::anyhow!("The OS secure vault is unavailable"))
}

pub fn get() -> Result<Option<Zeroizing<[u8; 32]>>> {
    match entry()?.get_secret() {
        Ok(mut bytes) => {
            if bytes.len() != 32 {
                bytes.zeroize();
                anyhow::bail!("The OS vault returned an invalid Squigit encryption master");
            }
            let mut key = Zeroizing::new([0; 32]);
            key.copy_from_slice(&bytes);
            bytes.zeroize();
            Ok(Some(key))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => anyhow::bail!("The OS secure vault is locked, unavailable, or denied access"),
    }
}

pub fn create() -> Result<Zeroizing<[u8; 32]>> {
    let mut generated = Zeroizing::new([0; 32]);
    OsRng.fill_bytes(&mut *generated);
    let entry = entry()?;
    entry
        .set_secret(&generated[..])
        .map_err(|_| anyhow::anyhow!("Could not store the encryption master in the OS vault"))?;
    let read_back =
        get()?.context("The OS vault did not return the encryption master it just stored")?;
    anyhow::ensure!(
        *generated == *read_back,
        "The OS vault did not preserve the encryption master"
    );
    Ok(read_back)
}
