use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use sqlg_core::{Connection, Secrets, Store};
use sqlg_mysql;
use sqlx::MySqlPool;
use uuid::Uuid;

pub struct Db {
    dir: PathBuf,
    pools: Mutex<HashMap<String, MySqlPool>>,
    explain: Mutex<HashMap<String, String>>,
    runs: Mutex<HashMap<String, RunControl>>,
}

struct RunControl {
    cancel: Arc<AtomicBool>,
    connection_id: Arc<AtomicU64>,
    connection_name: String,
}

pub struct RunHandle {
    pub run_id: String,
    pub cancel: Arc<AtomicBool>,
    pub connection_id: Arc<AtomicU64>,
}

impl Db {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            pools: Mutex::new(HashMap::new()),
            explain: Mutex::new(HashMap::new()),
            runs: Mutex::new(HashMap::new()),
        }
    }

    pub fn store(&self) -> Store {
        Store::open(self.dir.clone(), Secrets::from_env(&self.dir))
    }

    pub async fn pool_for(&self, name: &str) -> Result<MySqlPool, String> {
        if let Some(pool) = self.pools.lock().expect("pools").get(name).cloned() {
            return Ok(pool);
        }
        let store = self.store();
        let connection = store.get(name).map_err(|err| err.to_string())?;
        let password = store
            .password(name)
            .map_err(|err| err.to_string())?
            .unwrap_or_default();
        let pool = sqlg_mysql::connect_pool(&connection, &password).await?;
        self.pools
            .lock()
            .expect("pools")
            .insert(name.to_string(), pool.clone());
        Ok(pool)
    }

    pub fn forget_pool(&self, name: &str) {
        self.pools.lock().expect("pools").remove(name);
        let prefix = format!("{name}\n");
        self.explain
            .lock()
            .expect("explain")
            .retain(|key, _| !key.starts_with(&prefix));
    }

    pub fn track_run(&self, connection_name: &str) -> RunHandle {
        let run_id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let connection_id = Arc::new(AtomicU64::new(0));
        self.runs.lock().expect("runs").insert(
            run_id.clone(),
            RunControl {
                cancel: cancel.clone(),
                connection_id: connection_id.clone(),
                connection_name: connection_name.to_string(),
            },
        );
        RunHandle {
            run_id,
            cancel,
            connection_id,
        }
    }

    pub fn forget_run(&self, run_id: &str) {
        self.runs.lock().expect("runs").remove(run_id);
    }

    pub async fn cancel_run(&self, run_id: &str) {
        let control = {
            let runs = self.runs.lock().expect("runs");
            runs.get(run_id).map(|run| {
                (
                    run.cancel.clone(),
                    run.connection_id.clone(),
                    run.connection_name.clone(),
                )
            })
        };
        let Some((cancel, connection_id, name)) = control else {
            return;
        };
        cancel.store(true, Ordering::SeqCst);
        let id = connection_id.load(Ordering::SeqCst);
        if id == 0 {
            return;
        }
        if let Ok(pool) = self.pool_for(&name).await {
            let statement = format!("kill query {id}");
            let _ = sqlx::query(&statement).execute(&pool).await;
        }
    }

    pub async fn explain(&self, name: &str, sql: &str) -> Result<(), String> {
        let key = format!("{name}\n{sql}");
        if let Some(cached) = self.explain.lock().expect("explain").get(&key).cloned() {
            return if cached.is_empty() {
                Ok(())
            } else {
                Err(cached)
            };
        }
        let pool = self.pool_for(name).await?;
        match sqlg_mysql::explain_select(&pool, sql).await {
            Ok(_) => {
                self.explain
                    .lock()
                    .expect("explain")
                    .insert(key, String::new());
                Ok(())
            }
            Err(err) => {
                self.explain
                    .lock()
                    .expect("explain")
                    .insert(key, err.clone());
                Err(err)
            }
        }
    }

    pub async fn test_connection(
        connection: &Connection,
        password: &str,
    ) -> Result<String, String> {
        let pool = sqlg_mysql::connect_pool(connection, password).await?;
        sqlg_mysql::server_version(&pool).await
    }
}
