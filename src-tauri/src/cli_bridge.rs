use std::path::Path;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use sqlg_core::CliRequest;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::state::AppState;
use crate::windows::{self, front_label};

pub fn ingest(app: &AppHandle, args: &[String]) {
    let Some(path) = args
        .iter()
        .position(|arg| arg == "--cli-request")
        .and_then(|idx| args.get(idx + 1))
    else {
        return;
    };
    let text = std::fs::read_to_string(path).ok();
    let _ = std::fs::remove_file(path);
    let Some(text) = text else {
        return;
    };
    let Ok(request) = serde_json::from_str::<CliRequest>(&text) else {
        return;
    };
    let _ = std::fs::write(&request.ack, b"ok");
    let state = app.state::<AppState>();
    if request.target == "new-window" {
        let label = format!("win-{}", Uuid::new_v4());
        let _ = windows::create_window(app, &label);
        state
            .pending
            .lock()
            .expect("pending")
            .insert(label.clone(), request);
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.emit("cli-request", ());
        }
        return;
    }
    let label = front_label(app);
    state
        .pending
        .lock()
        .expect("pending")
        .insert(label.clone(), request);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.emit("cli-request", ());
    }
}

pub fn watch_connections(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let dir = state.data_dir.clone();
    let handle = app.clone();
    let mut watcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            let Ok(event) = result else {
                return;
            };
            let interesting = event
                .paths
                .iter()
                .any(|path| path.ends_with("connections.toml"));
            if interesting {
                let _ = handle.emit("connections-changed", ());
            }
        })
        .map_err(|err| err.to_string())?;
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    watcher
        .watch(Path::new(&dir), RecursiveMode::NonRecursive)
        .map_err(|err| err.to_string())?;
    state.watchers.lock().expect("watchers").push(watcher);
    Ok(())
}

pub fn watch_file(app: &AppHandle, path: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let handle = app.clone();
    let watched = path.clone();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            if result.is_ok() {
                let _ = handle.emit("file-changed", serde_json::json!({ "path": watched }));
            }
        })
        .map_err(|err| err.to_string())?;
    watcher
        .watch(Path::new(&path), RecursiveMode::NonRecursive)
        .map_err(|err| err.to_string())?;
    state
        .file_watchers
        .lock()
        .expect("file watchers")
        .insert(path, watcher);
    Ok(())
}

pub fn unwatch_file(app: &AppHandle, path: &str) {
    let state = app.state::<AppState>();
    state
        .file_watchers
        .lock()
        .expect("file watchers")
        .remove(path);
}
