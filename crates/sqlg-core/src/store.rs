use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

use crate::secrets::{SecretStore, Secrets};

pub fn account_for(name: &str) -> String {
    format!("conn:{name}")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connection {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default)]
    pub ssl: bool,
    #[serde(default)]
    pub explain_lint: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn default_port() -> u16 {
    3306
}

impl Connection {
    pub fn normalized(mut self) -> Result<Self, StoreError> {
        self.name = self.name.trim().to_string();
        self.host = self.host.trim().to_string();
        self.user = self.user.trim().to_string();
        if let Some(schema) = &self.schema {
            let schema = schema.trim().to_string();
            self.schema = if schema.is_empty() {
                None
            } else {
                Some(schema)
            };
        }
        if let Some(color) = &self.color {
            let color = color.trim().to_string();
            self.color = if color.is_empty() { None } else { Some(color) };
        }
        validate_name(&self.name)?;
        if self.host.is_empty() {
            return Err(StoreError::Invalid("host is required".into()));
        }
        if self.port == 0 {
            self.port = 3306;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionPatch {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub schema: Option<Option<String>>,
    pub ssl: Option<bool>,
    pub explain_lint: Option<bool>,
    pub color: Option<Option<String>>,
    pub rename: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("connection '{0}' already exists")]
    Conflict(String),
    #[error("connection '{0}' not found")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("config io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse: {0}")]
    Parse(String),
    #[error("keychain: {0}")]
    Keychain(String),
}

impl StoreError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Conflict(_) => 3,
            Self::NotFound(_) => 4,
            Self::Keychain(_) => 6,
            Self::Io(_) | Self::Parse(_) => 7,
            Self::Invalid(_) => 2,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConnectionsFile {
    #[serde(default)]
    connection: Vec<Connection>,
}

pub struct Store {
    dir: PathBuf,
    secrets: Secrets,
}

impl Store {
    pub fn open(dir: impl Into<PathBuf>, secrets: Secrets) -> Self {
        Self {
            dir: dir.into(),
            secrets,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self) -> Result<Vec<Connection>, StoreError> {
        Ok(self.read()?.connection)
    }

    pub fn get(&self, name: &str) -> Result<Connection, StoreError> {
        self.list()?
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| StoreError::NotFound(name.to_string()))
    }

    pub fn password(&self, name: &str) -> Result<Option<String>, StoreError> {
        self.secrets.get(&account_for(name))
    }

    pub fn add(&self, connection: Connection, password: &str) -> Result<(), StoreError> {
        let connection = connection.normalized()?;
        self.with_file(|file| {
            if file.connection.iter().any(|c| c.name == connection.name) {
                return Err(StoreError::Conflict(connection.name.clone()));
            }
            file.connection.push(connection.clone());
            Ok(())
        })?;
        if let Err(err) = self.secrets.set(&account_for(&connection.name), password) {
            let _ = self.with_file(|file| {
                file.connection.retain(|c| c.name != connection.name);
                Ok(())
            });
            return Err(err);
        }
        Ok(())
    }

    pub fn update(&self, name: &str, patch: ConnectionPatch) -> Result<Connection, StoreError> {
        let mut updated = None;
        let rename_to = patch.rename.clone();
        self.with_file(|file| {
            let Some(idx) = file.connection.iter().position(|c| c.name == name) else {
                return Err(StoreError::NotFound(name.to_string()));
            };
            if let Some(new_name) = &patch.rename {
                let new_name = new_name.trim().to_string();
                validate_name(&new_name)?;
                if new_name != file.connection[idx].name
                    && file.connection.iter().any(|c| c.name == new_name)
                {
                    return Err(StoreError::Conflict(new_name));
                }
            }
            let current = &mut file.connection[idx];
            if let Some(host) = &patch.host {
                current.host = host.trim().to_string();
            }
            if let Some(port) = patch.port {
                current.port = port;
            }
            if let Some(user) = &patch.user {
                current.user = user.trim().to_string();
            }
            if let Some(schema) = &patch.schema {
                current.schema = schema.clone().filter(|value| !value.trim().is_empty());
            }
            if let Some(ssl) = patch.ssl {
                current.ssl = ssl;
            }
            if let Some(flag) = patch.explain_lint {
                current.explain_lint = flag;
            }
            if let Some(color) = &patch.color {
                current.color = color.clone().filter(|value| !value.trim().is_empty());
            }
            if let Some(new_name) = &patch.rename {
                current.name = new_name.trim().to_string();
            }
            let normalized = current.clone().normalized()?;
            *current = normalized.clone();
            updated = Some(normalized);
            Ok(())
        })?;
        let updated = updated.ok_or_else(|| StoreError::NotFound(name.to_string()))?;
        if let Some(new_name) = rename_to.map(|n| n.trim().to_string())
            && new_name != name
        {
            let existing = self.secrets.get(&account_for(name))?;
            if let Some(secret) = existing {
                self.secrets.set(&account_for(&new_name), &secret)?;
                self.secrets.delete(&account_for(name))?;
            }
        }
        if let Some(password) = patch.password {
            self.secrets.set(&account_for(&updated.name), &password)?;
        }
        Ok(updated)
    }

    pub fn remove(&self, name: &str) -> Result<(), StoreError> {
        let mut found = false;
        self.with_file(|file| {
            let before = file.connection.len();
            file.connection.retain(|c| c.name != name);
            found = file.connection.len() != before;
            if found {
                Ok(())
            } else {
                Err(StoreError::NotFound(name.to_string()))
            }
        })?;
        self.secrets.delete(&account_for(name))?;
        Ok(())
    }

    fn config_path(&self) -> PathBuf {
        self.dir.join("connections.toml")
    }

    fn read(&self) -> Result<ConnectionsFile, StoreError> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(ConnectionsFile::default());
        }
        let text = std::fs::read_to_string(&path)?;
        toml::from_str(&text).map_err(|err| StoreError::Parse(err.to_string()))
    }

    fn with_file<T>(
        &self,
        body: impl FnOnce(&mut ConnectionsFile) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        std::fs::create_dir_all(&self.dir)?;
        let lock_path = self.dir.join("connections.lock");
        let lock = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.lock_exclusive()?;
        let mut file = self.read()?;
        let value = body(&mut file)?;
        atomic_write(
            &self.config_path(),
            toml::to_string_pretty(&file).unwrap_or_default().as_bytes(),
        )?;
        Ok(value)
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn validate_name(name: &str) -> Result<(), StoreError> {
    if name.is_empty()
        || name.len() > 64
        || name
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
    {
        return Err(StoreError::Invalid(
            "connection name must be 1-64 characters without slashes".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample(name: &str) -> Connection {
        Connection {
            name: name.into(),
            host: "127.0.0.1".into(),
            port: 3306,
            user: "root".into(),
            schema: Some("fleet".into()),
            ssl: false,
            explain_lint: false,
            color: None,
        }
    }

    #[test]
    fn round_trip_rename_and_delete() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path(), Secrets::memory());
        store.add(sample("dev"), "s3cret").unwrap();
        let text = std::fs::read_to_string(dir.path().join("connections.toml")).unwrap();
        assert!(text.contains("name = \"dev\""));
        assert!(!text.contains("s3cret"));
        assert_eq!(store.password("dev").unwrap().as_deref(), Some("s3cret"));

        let err = store.add(sample("dev"), "other").unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));

        let renamed = store
            .update(
                "dev",
                ConnectionPatch {
                    host: Some("db.internal".into()),
                    rename: Some("staging".into()),
                    password: Some("next".into()),
                    ..ConnectionPatch::default()
                },
            )
            .unwrap();
        assert_eq!(renamed.name, "staging");
        assert_eq!(renamed.host, "db.internal");
        assert!(store.get("dev").is_err());
        assert_eq!(store.password("staging").unwrap().as_deref(), Some("next"));
        assert!(store.password("dev").unwrap().is_none());

        store.remove("staging").unwrap();
        assert!(store.list().unwrap().is_empty());
        assert!(matches!(
            store.remove("staging"),
            Err(StoreError::NotFound(_))
        ));
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_temp() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.json");
        atomic_write(&path, b"{\"version\":1}").unwrap();
        atomic_write(&path, b"{\"version\":1,\"theme\":\"dark\"}").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\"version\":1,\"theme\":\"dark\"}"
        );
        assert!(!dir.path().join("session.tmp").exists());
    }
}
