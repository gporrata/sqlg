use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::windows;

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let about = PredefinedMenuItem::about(app, Some("About sqlg"), None)?;
    let install_cli = MenuItem::with_id(
        app,
        "app-install-cli",
        "Install 'sqlg' Command in PATH",
        true,
        None::<&str>,
    )?;
    let uninstall_cli = MenuItem::with_id(
        app,
        "app-uninstall-cli",
        "Uninstall 'sqlg' Command",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "app-quit", "Quit sqlg", true, Some("CmdOrCtrl+Q"))?;
    let app_sep_a = PredefinedMenuItem::separator(app)?;
    let app_sep_b = PredefinedMenuItem::separator(app)?;
    let app_menu = Submenu::with_items(
        app,
        "sqlg",
        true,
        &[
            &about,
            &app_sep_a,
            &install_cli,
            &uninstall_cli,
            &app_sep_b,
            &quit,
        ],
    )?;

    let new_tab = MenuItem::with_id(app, "file-new-tab", "New Tab", true, Some("CmdOrCtrl+T"))?;
    let new_window = MenuItem::with_id(
        app,
        "file-new-window",
        "New Window",
        true,
        Some("CmdOrCtrl+N"),
    )?;
    let open = MenuItem::with_id(app, "file-open", "Open…", true, Some("CmdOrCtrl+O"))?;
    let save = MenuItem::with_id(app, "file-save", "Save", true, Some("CmdOrCtrl+S"))?;
    let save_as = MenuItem::with_id(
        app,
        "file-save-as",
        "Save As…",
        true,
        Some("Shift+CmdOrCtrl+S"),
    )?;
    let save_sel = MenuItem::with_id(
        app,
        "file-save-selection",
        "Save Selection As…",
        true,
        None::<&str>,
    )?;
    let close_tab = MenuItem::with_id(
        app,
        "file-close-tab",
        "Close Tab",
        true,
        Some("CmdOrCtrl+W"),
    )?;
    let close_window = MenuItem::with_id(
        app,
        "file-close-window",
        "Close Window",
        true,
        Some("Shift+CmdOrCtrl+W"),
    )?;
    let connections = MenuItem::with_id(
        app,
        "file-connections",
        "Connections…",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let file_sep_a = PredefinedMenuItem::separator(app)?;
    let file_sep_b = PredefinedMenuItem::separator(app)?;
    let file_sep_c = PredefinedMenuItem::separator(app)?;
    let file_sep_d = PredefinedMenuItem::separator(app)?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new_tab,
            &new_window,
            &file_sep_a,
            &open,
            &file_sep_b,
            &save,
            &save_as,
            &save_sel,
            &file_sep_c,
            &close_tab,
            &close_window,
            &file_sep_d,
            &connections,
        ],
    )?;

    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    let edit_sep_a = PredefinedMenuItem::separator(app)?;
    let edit_sep_b = PredefinedMenuItem::separator(app)?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &edit_sep_a,
            &cut,
            &copy,
            &paste,
            &edit_sep_b,
            &select_all,
        ],
    )?;

    let light = MenuItem::with_id(app, "view-theme-light", "Light", true, None::<&str>)?;
    let dark = MenuItem::with_id(app, "view-theme-dark", "Dark", true, None::<&str>)?;
    let view_menu = Submenu::with_items(app, "View", true, &[&light, &dark])?;

    let run = MenuItem::with_id(app, "query-run", "Run", true, Some("CmdOrCtrl+Return"))?;
    let run_stmt = MenuItem::with_id(
        app,
        "query-run-statement",
        "Run Statement",
        true,
        Some("Shift+CmdOrCtrl+Return"),
    )?;
    let stop = MenuItem::with_id(app, "query-stop", "Stop", true, Some("CmdOrCtrl+."))?;
    let query_menu = Submenu::with_items(app, "Query", true, &[&run, &run_stmt, &stop])?;

    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let window_menu = Submenu::with_items(app, "Window", true, &[&minimize])?;

    let menu = Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &query_menu,
            &window_menu,
        ],
    )?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        let id = event.id().as_ref().to_string();
        match id.as_str() {
            "app-quit" => request_quit(app),
            "view-theme-light" => {
                windows::apply_theme(app, "light");
                let _ = app.emit("theme-changed", "light");
            }
            "view-theme-dark" => {
                windows::apply_theme(app, "dark");
                let _ = app.emit("theme-changed", "dark");
            }
            _ => emit_menu(app, &id),
        }
    });
    Ok(())
}

fn emit_menu(app: &AppHandle, id: &str) {
    let windows = app.webview_windows();
    let focused = windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false));
    if let Some(window) = focused.or_else(|| windows.values().next()) {
        let _ = window.emit("menu", id);
    }
}

pub fn request_quit(app: &AppHandle) {
    let state = app.state::<AppState>();
    state
        .quitting
        .store(true, std::sync::atomic::Ordering::SeqCst);
    state
        .quit_acks
        .store(0, std::sync::atomic::Ordering::SeqCst);
    let _ = app.emit("prepare-quit", ());
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let state = handle.state::<AppState>();
        if !state
            .allow_close
            .swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            handle.exit(0);
        }
    });
}
