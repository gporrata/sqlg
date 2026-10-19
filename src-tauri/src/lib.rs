mod cli_bridge;
mod commands;
mod db;
mod menu;
mod state;
mod windows;

use tauri::{Emitter, Manager};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            cli_bridge::ingest(app, &args);
        }))
        .setup(|app| {
            let dir = sqlg_core::data_dir();
            let _ = std::fs::create_dir_all(&dir);
            let state = AppState::new(dir);
            let session = state.load_session();
            app.manage(state);
            menu::install(app.handle())?;
            windows::restore(app.handle(), &session.windows, &session.theme)?;
            cli_bridge::watch_connections(app.handle())?;
            let args: Vec<String> = std::env::args().skip(1).collect();
            cli_bridge::ingest(app.handle(), &args);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                let allowed = state.allow_close.load(std::sync::atomic::Ordering::SeqCst)
                    || state
                        .closing
                        .lock()
                        .expect("closing")
                        .contains(window.label());
                if !allowed {
                    api.prevent_close();
                    let quitting = state.quitting.load(std::sync::atomic::Ordering::SeqCst);
                    let _ = window.emit("native-close", quitting);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_connections,
            commands::add_connection,
            commands::update_connection,
            commands::remove_connection,
            commands::test_connection,
            commands::load_schema,
            commands::connection_ping,
            commands::run_query,
            commands::cancel_query,
            commands::forget_run,
            commands::lint_sql_cmd,
            commands::statement_at_cmd,
            commands::explain_sql,
            commands::read_text_file,
            commands::write_text_file,
            commands::watch_file,
            commands::unwatch_file,
            commands::load_session,
            commands::load_scratch,
            commands::save_scratch,
            commands::upsert_window,
            commands::remove_window,
            commands::set_theme,
            commands::place_window,
            commands::create_window,
            commands::take_pending,
            commands::is_quitting,
            commands::hold_quit,
            commands::cancel_quit,
            commands::ack_quit,
            commands::finish_quit,
            commands::permit_close,
            commands::request_quit,
            commands::install_cli,
            commands::uninstall_cli,
            commands::deliver_args
        ])
        .build(tauri::generate_context!())
        .expect("error while building sqlg")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let state = app.state::<AppState>();
                if !state.allow_close.load(std::sync::atomic::Ordering::SeqCst) {
                    api.prevent_exit();
                    menu::request_quit(app);
                }
            }
        });
}
