use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::Serialize;
use sqlg_core::{
    CliRequest, Connection, ConnectionPatch, Diagnostic, LintCatalog, WindowSnapshot, lint_sql,
    statement_at,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::cli_bridge::{self, ingest};
use crate::db::{self, QueryEvent, SchemaSnapshot};
use crate::menu;
use crate::state::AppState;
use crate::windows;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile {
    pub text: String,
    pub newline: String,
    pub bom: bool,
}

#[tauri::command]
pub fn list_connections(state: State<'_, AppState>) -> Result<Vec<Connection>, String> {
    state.store().list().map_err(|err| err.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_connection(
    state: State<'_, AppState>,
    name: String,
    host: String,
    port: u16,
    user: String,
    password: String,
    schema: Option<String>,
    ssl: bool,
    explain_lint: bool,
    color: Option<String>,
) -> Result<(), String> {
    let connection = Connection {
        name,
        host,
        port,
        user,
        schema,
        ssl,
        explain_lint,
        color,
    };
    state
        .store()
        .add(connection, &password)
        .map_err(|err| err.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_connection(
    state: State<'_, AppState>,
    name: String,
    host: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    schema: Option<String>,
    schema_set: bool,
    ssl: Option<bool>,
    explain_lint: Option<bool>,
    color: Option<String>,
    color_set: bool,
    rename: Option<String>,
) -> Result<Connection, String> {
    let patch = ConnectionPatch {
        host,
        port,
        user,
        schema: schema_set.then_some(schema),
        ssl,
        explain_lint,
        color: color_set.then_some(color),
        rename: rename.clone(),
        password,
    };
    let updated = state
        .store()
        .update(&name, patch)
        .map_err(|err| err.to_string())?;
    let mut pools = state.pools.lock().expect("pools");
    pools.remove(&name);
    if let Some(new_name) = rename {
        pools.remove(new_name.trim());
    }
    pools.remove(&updated.name);
    Ok(updated)
}

#[tauri::command]
pub fn remove_connection(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.store().remove(&name).map_err(|err| err.to_string())?;
    state.pools.lock().expect("pools").remove(&name);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn test_connection(
    state: State<'_, AppState>,
    name: Option<String>,
    host: String,
    port: u16,
    user: String,
    password: String,
    schema: Option<String>,
    ssl: bool,
) -> Result<String, String> {
    let mut password = password;
    if password.is_empty()
        && let Some(saved) = name.as_deref()
    {
        password = state
            .store()
            .password(saved)
            .map_err(|err| err.to_string())?
            .unwrap_or_default();
    }
    let connection = Connection {
        name: name.unwrap_or_else(|| "test".into()),
        host,
        port,
        user,
        schema,
        ssl,
        explain_lint: false,
        color: None,
    };
    let pool = db::connect_pool(&connection, &password).await?;
    db::server_version(&pool).await
}

#[tauri::command]
pub async fn load_schema(
    state: State<'_, AppState>,
    name: String,
) -> Result<SchemaSnapshot, String> {
    let pool = pool_for(&state, &name).await?;
    db::load_schema(&pool).await
}

#[tauri::command]
pub async fn connection_ping(state: State<'_, AppState>, name: String) -> Result<bool, String> {
    let pool = pool_for(&state, &name).await?;
    sqlx::query("select 1")
        .execute(&pool)
        .await
        .map(|_| true)
        .map_err(|err| db::format_sqlx_error(&err))
}

#[tauri::command]
pub async fn run_query(
    app: AppHandle,
    state: State<'_, AppState>,
    channel: Channel<QueryEvent>,
    connection_name: String,
    sql: String,
) -> Result<String, String> {
    let pool = pool_for(&state, &connection_name).await?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    let connection_id = Arc::new(AtomicU64::new(0));
    state.runs.lock().expect("runs").insert(
        run_id.clone(),
        crate::state::RunControl {
            cancel: cancel.clone(),
            connection_id: connection_id.clone(),
            connection_name: connection_name.clone(),
        },
    );
    let tracked = run_id.clone();
    tauri::async_runtime::spawn(async move {
        db::execute_script(pool, tracked.clone(), sql, cancel, connection_id, channel).await;
        app.state::<AppState>()
            .runs
            .lock()
            .expect("runs")
            .remove(&tracked);
    });
    Ok(run_id)
}

#[tauri::command]
pub async fn cancel_query(state: State<'_, AppState>, run_id: String) -> Result<(), String> {
    let control = {
        let runs = state.runs.lock().expect("runs");
        runs.get(&run_id).map(|run| {
            (
                run.cancel.clone(),
                run.connection_id.clone(),
                run.connection_name.clone(),
            )
        })
    };
    let Some((cancel, connection_id, name)) = control else {
        return Ok(());
    };
    cancel.store(true, Ordering::SeqCst);
    if let Ok(pool) = pool_for(&state, &name).await {
        let id = connection_id.load(Ordering::SeqCst);
        if id != 0 {
            let statement = format!("kill query {id}");
            let _ = sqlx::query(&statement).execute(&pool).await;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn forget_run(state: State<'_, AppState>, run_id: String) {
    state.runs.lock().expect("runs").remove(&run_id);
}

#[tauri::command]
pub fn lint_sql_cmd(sql: String, catalog: LintCatalog) -> Vec<Diagnostic> {
    lint_sql(&sql, &catalog)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatementRange {
    sql: String,
    start: usize,
    end: usize,
}

#[tauri::command]
pub fn statement_at_cmd(sql: String, cursor: usize) -> Option<StatementRange> {
    statement_at(&sql, cursor).map(|statement| StatementRange {
        sql: statement.sql,
        start: statement.start,
        end: statement.end,
    })
}

#[tauri::command]
pub async fn explain_sql(
    state: State<'_, AppState>,
    name: String,
    sql: String,
) -> Result<String, String> {
    let key = format!("{name}\n{sql}");
    if let Some(cached) = state
        .explain_cache
        .lock()
        .expect("explain")
        .get(&key)
        .cloned()
    {
        return Ok(cached);
    }
    let pool = pool_for(&state, &name).await?;
    let text = db::explain_select(&pool, &sql).await?;
    state
        .explain_cache
        .lock()
        .expect("explain")
        .insert(key, text.clone());
    Ok(text)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<TextFile, String> {
    let bytes = std::fs::read(&path).map_err(|err| err.to_string())?;
    let bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
    let body = if bom { &bytes[3..] } else { &bytes };
    let text = String::from_utf8(body.to_vec()).map_err(|_| "file is not UTF-8".to_string())?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    Ok(TextFile {
        text: text.replace("\r\n", "\n"),
        newline: newline.into(),
        bom,
    })
}

#[tauri::command]
pub fn write_text_file(
    path: String,
    text: String,
    newline: String,
    bom: bool,
) -> Result<(), String> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let nl = if newline == "\r\n" { "\r\n" } else { "\n" };
    let mut body = text.replace("\r\n", "\n").replace('\n', nl);
    if bom {
        body.insert(0, '\u{feff}');
    }
    let mut file = std::fs::File::create(&path).map_err(|err| err.to_string())?;
    file.write_all(body.as_bytes())
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn watch_file(app: AppHandle, path: String) -> Result<(), String> {
    cli_bridge::watch_file(&app, path)
}

#[tauri::command]
pub fn unwatch_file(app: AppHandle, path: String) {
    cli_bridge::unwatch_file(&app, &path);
}

#[tauri::command]
pub fn load_session(state: State<'_, AppState>) -> sqlg_core::SessionDocument {
    state.load_session()
}

#[tauri::command]
pub fn upsert_window(state: State<'_, AppState>, snapshot: WindowSnapshot) -> Result<(), String> {
    state.update_session(|doc| {
        if let Some(existing) = doc
            .windows
            .iter_mut()
            .find(|window| window.label == snapshot.label)
        {
            *existing = snapshot.clone();
        } else {
            doc.windows.push(snapshot.clone());
        }
    })
}

#[tauri::command]
pub fn remove_window(state: State<'_, AppState>, label: String) -> Result<(), String> {
    state.update_session(|doc| {
        doc.windows.retain(|window| window.label != label);
        if doc.front_window.as_deref() == Some(label.as_str()) {
            doc.front_window = None;
        }
    })
}

#[tauri::command]
pub fn set_theme(app: AppHandle, state: State<'_, AppState>, theme: String) -> Result<(), String> {
    let theme = if theme == "dark" { "dark" } else { "light" };
    state.update_session(|doc| {
        doc.theme = theme.to_string();
    })?;
    windows::apply_theme(&app, theme);
    let _ = app.emit("theme-changed", theme);
    Ok(())
}

#[tauri::command]
pub fn place_window(app: AppHandle, snapshot: WindowSnapshot) -> Result<(), String> {
    windows::place(&app, &snapshot)
}

#[tauri::command]
pub fn create_window(app: AppHandle) -> Result<String, String> {
    let label = format!("win-{}", uuid::Uuid::new_v4());
    windows::create_window(&app, &label)?;
    Ok(label)
}

#[tauri::command]
pub fn take_pending(state: State<'_, AppState>, label: String) -> Option<CliRequest> {
    state.pending.lock().expect("pending").remove(&label)
}

#[tauri::command]
pub fn is_quitting(state: State<'_, AppState>) -> bool {
    state.quitting.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn ack_quit(app: AppHandle, state: State<'_, AppState>) {
    let acks = state.quit_acks.fetch_add(1, Ordering::SeqCst) + 1;
    let windows = app.webview_windows().len().max(1);
    if acks >= windows {
        state.allow_close.store(true, Ordering::SeqCst);
        app.exit(0);
    }
}

#[tauri::command]
pub fn finish_quit(app: AppHandle, state: State<'_, AppState>) {
    state.allow_close.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
pub fn permit_close(app: AppHandle, state: State<'_, AppState>, label: String) {
    state.closing.lock().expect("closing").insert(label.clone());
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.destroy();
    }
}

#[tauri::command]
pub fn request_quit(app: AppHandle) {
    menu::request_quit(&app);
}

#[tauri::command]
pub fn install_cli() -> Result<String, String> {
    let source = cli_binary()
        .ok_or_else(|| "sqlg command binary was not found beside the app".to_string())?;
    let system = PathBuf::from("/usr/local/bin/sqlg");
    match link_cli(&source, &system) {
        Ok(()) => Ok(format!("linked {}", system.display())),
        Err(err) => {
            let local = local_bin();
            if let Some(parent) = local.parent() {
                std::fs::create_dir_all(parent).map_err(|io| io.to_string())?;
            }
            link_cli(&source, &local).map_err(|_| err)?;
            Ok(format!(
                "linked {}. Add {} to PATH if it is not already.",
                local.display(),
                local
                    .parent()
                    .unwrap_or(Path::new("~/.local/bin"))
                    .display()
            ))
        }
    }
}

#[tauri::command]
pub fn uninstall_cli() -> Result<String, String> {
    let mut removed = Vec::new();
    for path in [PathBuf::from("/usr/local/bin/sqlg"), local_bin()] {
        if path.symlink_metadata().is_ok() {
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(path.display().to_string()),
                Err(_) => {
                    let script = format!(
                        "do shell script \"rm -f '{}'\" with administrator privileges",
                        path.display()
                    );
                    let status = Command::new("osascript")
                        .arg("-e")
                        .arg(script)
                        .status()
                        .map_err(|err| err.to_string())?;
                    if status.success() {
                        removed.push(path.display().to_string());
                    }
                }
            }
        }
    }
    if removed.is_empty() {
        Ok("sqlg command was not installed".into())
    } else {
        Ok(format!("removed {}", removed.join(", ")))
    }
}

#[tauri::command]
pub fn deliver_args(app: AppHandle, args: Vec<String>) {
    ingest(&app, &args);
}

fn local_bin() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local/bin/sqlg")
}

fn cli_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let beside = exe.parent()?.join("sqlg");
    if beside.is_file() {
        return Some(beside);
    }
    None
}

fn link_cli(source: &Path, dest: &Path) -> Result<(), String> {
    if dest.symlink_metadata().is_ok() {
        std::fs::remove_file(dest).map_err(|err| err.to_string())?;
    }
    match std::os::unix::fs::symlink(source, dest) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            let script = format!(
                "do shell script \"ln -sfn '{}' '{}'\" with administrator privileges",
                escape_osascript(&source.display().to_string()),
                escape_osascript(&dest.display().to_string())
            );
            let status = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .status()
                .map_err(|os| os.to_string())?;
            if status.success() {
                Ok(())
            } else {
                Err("administrator install was declined".into())
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

fn escape_osascript(value: &str) -> String {
    value.replace('\'', "'\\''")
}

async fn pool_for(state: &AppState, name: &str) -> Result<sqlx::mysql::MySqlPool, String> {
    if let Some(pool) = state.pools.lock().expect("pools").get(name).cloned() {
        return Ok(pool);
    }
    let store = state.store();
    let connection = store.get(name).map_err(|err| err.to_string())?;
    let password = store
        .password(name)
        .map_err(|err| err.to_string())?
        .unwrap_or_default();
    let pool = db::connect_pool(&connection, &password).await?;
    state
        .pools
        .lock()
        .expect("pools")
        .insert(name.to_string(), pool.clone());
    Ok(pool)
}
