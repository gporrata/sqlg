use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};

use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use sqlg_core::{CliRequest, Secrets, SessionDocument, Store, atomic_write};
use sqlx::mysql::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScratchTab {
    pub id: String,
    #[serde(default)]
    pub window: String,
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default)]
    pub sql: String,
    #[serde(default)]
    pub cursor: u32,
    #[serde(default)]
    pub layout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScratchDocument {
    #[serde(default)]
    tabs: Vec<ScratchTab>,
}

pub struct RunControl {
    pub cancel: Arc<AtomicBool>,
    pub connection_id: Arc<AtomicU64>,
    pub connection_name: String,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub pools: Mutex<HashMap<String, MySqlPool>>,
    pub runs: Mutex<HashMap<String, RunControl>>,
    pub pending: Mutex<HashMap<String, CliRequest>>,
    pub explain_cache: Mutex<HashMap<String, String>>,
    pub watchers: Mutex<Vec<RecommendedWatcher>>,
    pub file_watchers: Mutex<HashMap<String, RecommendedWatcher>>,
    pub quitting: AtomicBool,
    pub quit_hold: AtomicBool,
    pub allow_close: AtomicBool,
    pub closing: Mutex<HashSet<String>>,
    pub quit_acks: AtomicUsize,
    pub session_lock: Mutex<()>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            pools: Mutex::new(HashMap::new()),
            runs: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            explain_cache: Mutex::new(HashMap::new()),
            watchers: Mutex::new(Vec::new()),
            file_watchers: Mutex::new(HashMap::new()),
            quitting: AtomicBool::new(false),
            quit_hold: AtomicBool::new(false),
            allow_close: AtomicBool::new(false),
            closing: Mutex::new(HashSet::new()),
            quit_acks: AtomicUsize::new(0),
            session_lock: Mutex::new(()),
        }
    }

    pub fn store(&self) -> Store {
        Store::open(&self.data_dir, Secrets::from_env(&self.data_dir))
    }

    pub fn session_path(&self) -> PathBuf {
        self.data_dir.join("session.json")
    }

    pub fn load_session(&self) -> SessionDocument {
        load_session_file(&self.session_path())
    }

    pub fn scratch_path(&self) -> PathBuf {
        self.data_dir.join("scratch.json")
    }

    pub fn load_scratch(&self) -> Vec<ScratchTab> {
        let Ok(text) = std::fs::read_to_string(self.scratch_path()) else {
            return Vec::new();
        };
        serde_json::from_str::<ScratchDocument>(&text)
            .unwrap_or_default()
            .tabs
    }

    pub fn save_scratch(&self, window: &str, tabs: Vec<ScratchTab>) -> Result<(), String> {
        let _guard = self.session_lock.lock().expect("session lock");
        let mut doc = serde_json::from_str::<ScratchDocument>(
            &std::fs::read_to_string(self.scratch_path()).unwrap_or_default(),
        )
        .unwrap_or_default();
        doc.tabs.retain(|tab| tab.window != window);
        doc.tabs.extend(tabs.into_iter().filter_map(|mut tab| {
            if tab.sql.is_empty() {
                return None;
            }
            tab.window = window.to_string();
            Some(tab)
        }));
        let bytes = serde_json::to_vec_pretty(&doc).map_err(|err| err.to_string())?;
        atomic_write(&self.scratch_path(), &bytes).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn update_session<T>(
        &self,
        body: impl FnOnce(&mut SessionDocument) -> T,
    ) -> Result<T, String> {
        let _guard = self.session_lock.lock().expect("session lock");
        let mut doc = load_session_file(&self.session_path());
        let value = body(&mut doc);
        let bytes = serde_json::to_vec_pretty(&doc).map_err(|err| err.to_string())?;
        atomic_write(&self.session_path(), &bytes).map_err(|err| err.to_string())?;
        Ok(value)
    }
}

pub fn load_session_file(path: &Path) -> SessionDocument {
    let Ok(text) = std::fs::read_to_string(path) else {
        return SessionDocument::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}
