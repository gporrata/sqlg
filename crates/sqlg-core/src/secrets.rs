use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::store::StoreError;

#[cfg(target_os = "macos")]
use crate::keychain_macos::Keychain;
#[cfg(not(target_os = "macos"))]
use crate::keychain_stub::Keychain;

/// Where connection passwords live. Production uses the macOS Keychain.
/// `SQLG_SECRET_BACKEND=file` is a test-only override so CLI tests do not
/// touch the login keychain.
pub enum Secrets {
    Keyring(Keychain),
    Unavailable(String),
    Memory(Mutex<HashMap<String, String>>),
    File(PathBuf),
}

impl Secrets {
    pub fn from_env(dir: &Path) -> Self {
        match std::env::var("SQLG_SECRET_BACKEND").as_deref() {
            Ok("file") => Self::File(dir.join("secrets.json")),
            Ok("memory") => Self::memory(),
            _ => match Keychain::user_default() {
                Ok(keychain) => Self::Keyring(keychain),
                Err(err) => Self::Unavailable(err.to_string()),
            },
        }
    }

    pub fn memory() -> Self {
        Self::Memory(Mutex::new(HashMap::new()))
    }
}

pub trait SecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, StoreError>;
    fn set(&self, account: &str, secret: &str) -> Result<(), StoreError>;
    fn delete(&self, account: &str) -> Result<(), StoreError>;
}

impl SecretStore for Secrets {
    fn get(&self, account: &str) -> Result<Option<String>, StoreError> {
        match self {
            Self::Memory(map) => Ok(map.lock().expect("secrets lock").get(account).cloned()),
            Self::File(path) => Ok(read_file(path)?.get(account).cloned()),
            Self::Keyring(keychain) => keychain.get(account),
            Self::Unavailable(message) => Err(StoreError::Keychain(message.clone())),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), StoreError> {
        match self {
            Self::Memory(map) => {
                map.lock()
                    .expect("secrets lock")
                    .insert(account.to_string(), secret.to_string());
                Ok(())
            }
            Self::File(path) => {
                let mut all = read_file(path)?;
                all.insert(account.to_string(), secret.to_string());
                write_file(path, &all)
            }
            Self::Keyring(keychain) => keychain.set(account, secret),
            Self::Unavailable(message) => Err(StoreError::Keychain(message.clone())),
        }
    }

    fn delete(&self, account: &str) -> Result<(), StoreError> {
        match self {
            Self::Memory(map) => {
                map.lock().expect("secrets lock").remove(account);
                Ok(())
            }
            Self::File(path) => {
                let mut all = read_file(path)?;
                all.remove(account);
                write_file(path, &all)
            }
            Self::Keyring(keychain) => keychain.delete(account),
            Self::Unavailable(message) => Err(StoreError::Keychain(message.clone())),
        }
    }
}

fn read_file(path: &Path) -> Result<HashMap<String, String>, StoreError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = std::fs::read(path)?;
    let parsed: SecretFile = serde_json::from_slice(&bytes).unwrap_or_default();
    Ok(parsed.entries)
}

fn write_file(path: &Path, entries: &HashMap<String, String>) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(&SecretFile {
        entries: entries.clone(),
    })
    .map_err(|err| StoreError::Io(std::io::Error::other(err)))?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&body)?;
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
struct SecretFile {
    entries: HashMap<String, String>,
}

trait OpenOptionsExt {
    fn mode(&mut self, mode: u32) -> &mut Self;
}

impl OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::mode(self, mode)
    }
}
