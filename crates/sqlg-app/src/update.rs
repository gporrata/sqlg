use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use iced::widget::text_editor::{Action, Content, Edit};
use iced::{Point, Size, Task, window};
use sqlg_core::{Connection, ConnectionPatch, WindowSnapshot};
use sqlg_mysql::QueryEvent;

use crate::db::Db;
use crate::files::{self, TextFile};
use crate::ipc::Incoming;
use crate::logic;
use crate::message::{DragKind, Field, FilePurpose, Message};
use crate::state::{App, ClosePrompt, ConnForm, Drag, GuiWindow, Modal, Tab};

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Nop => Task::none(),
        Message::Tick => on_tick(app),
        Message::ApplyStartup => apply_startup(app),
        Message::Edit {
            window,
            tab,
            action,
        } => on_edit(app, window, &tab, action),
        Message::Undo { window, tab } => on_undo(app, window, &tab, true),
        Message::Redo { window, tab } => on_undo(app, window, &tab, false),
        Message::Run {
            window,
            tab,
            statement,
        } => on_run(app, window, tab, statement),
        Message::Cancel { window, tab } => on_cancel(app, window, &tab),
        Message::Query {
            window,
            tab,
            run_id,
            event,
        } => on_query(app, window, &tab, &run_id, event),
        Message::NewTab(window) => on_new_tab(app, window),
        Message::NewWindow => {
            if app.blocks_shortcuts() {
                return Task::none();
            }
            open_blank(app)
        }
        Message::CloseTab(window) => {
            if app.blocks_shortcuts() {
                return Task::none();
            }
            request_close_tab(app, window)
        }
        Message::CloseWindow(window) => {
            if app.blocks_shortcuts() {
                return Task::none();
            }
            request_close_window(app, window)
        }
        Message::SelectTab { window, index } => {
            if let Some(gui) = app.window_mut(window) {
                if index < gui.tabs.len() {
                    gui.active = index;
                    gui.file_menu = false;
                }
            }
            Task::none()
        }
        Message::SetConnection { window, tab, name } => on_set_connection(app, window, &tab, name),
        Message::SetLayout { window, layout } => {
            if let Some(gui) = app.window_mut(window)
                && let Some(tab) = gui.active_tab_mut()
                && logic::known_layout(&layout)
            {
                tab.layout = layout;
            }
            app.save_session();
            Task::none()
        }
        Message::ToggleTheme => {
            app.theme_name = if app.theme_name == "dark" {
                "light".into()
            } else {
                "dark".into()
            };
            app.save_session();
            Task::none()
        }
        Message::ToggleFileMenu(window) => {
            if let Some(gui) = app.window_mut(window) {
                gui.file_menu = !gui.file_menu;
            }
            Task::none()
        }
        Message::Open(window) => {
            if app.blocks_shortcuts() {
                return Task::none();
            }
            hide_menu(app, window);
            let tab = active_tab_id(app, window).unwrap_or_default();
            pick_file(window, tab, FilePurpose::Open, false)
        }
        Message::Save(window) => on_save(app, window),
        Message::SaveAs(window) => on_save_as(app, window),
        Message::SaveSelection(window) => on_save_selection(app, window),
        Message::FilePicked {
            window,
            tab,
            purpose,
            path,
        } => on_file_picked(app, window, &tab, purpose, path),
        Message::Drop { window, path } => on_drop(app, window, path),
        Message::ReloadDisk { window, tab } => on_reload_disk(app, window, &tab),
        Message::DismissDisk { window, tab } => {
            if let Some(tab) = app.tab_mut(window, &tab) {
                tab.disk_notice = false;
            }
            Task::none()
        }
        Message::InsertText { window, tab, text } => {
            if let Some(tab) = app.tab_mut(window, &tab) {
                tab.remember_undo();
                tab.content
                    .perform(Action::Edit(Edit::Paste(Arc::new(text))));
            }
            let lint = schedule_lint(app, window, &tab);
            lint
        }
        Message::ToggleExpand { window, key } => {
            if let Some(gui) = app.window_mut(window) {
                if !gui.expanded.insert(key.clone()) {
                    gui.expanded.remove(&key);
                }
            }
            Task::none()
        }
        Message::SchemaLoaded { name, result } => {
            app.schema_loading.remove(&name);
            match result {
                Ok(snapshot) => {
                    app.schema_errors.remove(&name);
                    app.schemas.insert(name, snapshot);
                }
                Err(err) => {
                    app.schema_errors.insert(name, err);
                }
            }
            Task::none()
        }
        Message::Pinged { name, ok } => {
            app.pings.insert(name, ok);
            Task::none()
        }
        Message::ReloadSchema { window } => {
            let name = app
                .window(window)
                .and_then(|gui| gui.active_tab())
                .and_then(|tab| tab.connection_name.clone());
            if let Some(name) = name {
                app.schemas.remove(&name);
                app.schema_errors.remove(&name);
                app.schema_loading.remove(&name);
                app.db.forget_pool(&name);
                return ensure_schema(app, &name);
            }
            Task::none()
        }
        Message::LintReady {
            window,
            tab,
            generation,
            diagnostics,
        } => on_lint_ready(app, window, &tab, generation, diagnostics),
        Message::ExplainReady {
            window,
            tab,
            generation,
            result,
        } => on_explain_ready(app, window, &tab, generation, result),
        Message::ApplyFix {
            window,
            tab,
            from,
            to,
            text,
        } => on_apply_fix(app, window, &tab, from, to, &text),
        Message::Copy(text) => {
            app.status = "Copied".into();
            iced::clipboard::write(text)
        }
        Message::ShowMore { window, tab } => {
            if let Some(tab) = app.tab_mut(window, &tab) {
                tab.show_rows = tab.show_rows.saturating_add(logic::VISIBLE_ROWS);
            }
            Task::none()
        }
        Message::OpenConnections(window) => {
            if app.blocks_shortcuts() {
                return Task::none();
            }
            hide_menu(app, window);
            app.modal = Modal::Connections(ConnForm::blank());
            Task::none()
        }
        Message::FormNew => {
            app.modal = Modal::Connections(ConnForm::blank());
            Task::none()
        }
        Message::FormPick(name) => {
            let form = app
                .connection(&name)
                .map(ConnForm::from_connection)
                .unwrap_or_else(ConnForm::blank);
            app.modal = Modal::Connections(form);
            Task::none()
        }
        Message::FormInput { field, value } => {
            if let Modal::Connections(form) = &mut app.modal {
                match field {
                    Field::Name => form.name = value,
                    Field::Host => form.host = value,
                    Field::Port => form.port = value,
                    Field::User => form.user = value,
                    Field::Password => form.password = value,
                    Field::Schema => form.schema = value,
                    Field::Color => form.color = value,
                }
            }
            Task::none()
        }
        Message::FormFlag { ssl, explain } => {
            if let Modal::Connections(form) = &mut app.modal {
                if let Some(ssl) = ssl {
                    form.ssl = ssl;
                }
                if let Some(explain) = explain {
                    form.explain_lint = explain;
                }
            }
            Task::none()
        }
        Message::FormSave => save_form(app),
        Message::FormTest => test_form(app),
        Message::FormDelete => delete_form(app),
        Message::FormTested(result) => {
            if let Modal::Connections(form) = &mut app.modal {
                form.status = match result {
                    Ok(version) => format!("connected · {version}"),
                    Err(err) => err,
                };
            }
            Task::none()
        }
        Message::DismissModal => {
            app.modal = Modal::None;
            Task::none()
        }
        Message::CloseChoice { save } => apply_close_choice(app, save),
        Message::InstallCli => Task::perform(
            async { tokio::task::spawn_blocking(files::install_cli).await },
            |result| match result {
                Ok(inner) => Message::CliResult(inner),
                Err(err) => Message::CliResult(Err(err.to_string())),
            },
        ),
        Message::UninstallCli => Task::perform(
            async { tokio::task::spawn_blocking(files::uninstall_cli).await },
            |result| match result {
                Ok(inner) => Message::CliResult(inner),
                Err(err) => Message::CliResult(Err(err.to_string())),
            },
        ),
        Message::CliResult(result) => {
            app.status = match result {
                Ok(text) => text,
                Err(err) => err,
            };
            Task::none()
        }
        Message::StartDrag { window, kind } => {
            let origin_size = drag_origin(app, window, &kind);
            app.drag = Some(Drag {
                window,
                kind,
                origin: None,
                origin_size,
            });
            Task::none()
        }
        Message::DragMove(point) => on_drag(app, point),
        Message::DragEnd => {
            if app.drag.take().is_some() {
                app.save_session();
            }
            Task::none()
        }
        Message::Quit => request_quit(app),
        Message::Escape(window) => {
            if !matches!(app.modal, Modal::None) {
                app.modal = Modal::None;
            } else if let Some(gui) = app.window_mut(window) {
                gui.file_menu = false;
            }
            Task::none()
        }
        Message::WindowOpened {
            window,
            position,
            size,
        } => {
            if let Some(gui) = app.window_mut(window) {
                if let Some(position) = position {
                    gui.x = position.x as f64;
                    gui.y = position.y as f64;
                }
                gui.width = size.width as f64;
                gui.height = size.height as f64;
            }
            Task::none()
        }
        Message::WindowClosed(window) => {
            let present = app.window(window).is_some();
            if present {
                app.windows.retain(|gui| gui.id != window);
                if app.front == Some(window) {
                    app.front = app.windows.last().map(|gui| gui.id);
                }
                app.save_session();
                if app.windows.is_empty() {
                    return iced::exit();
                }
            }
            Task::none()
        }
        Message::Moved { window, point } => {
            if let Some(gui) = app.window_mut(window) {
                gui.x = point.x as f64;
                gui.y = point.y as f64;
            }
            Task::none()
        }
        Message::Resized { window, size } => {
            if let Some(gui) = app.window_mut(window) {
                gui.width = size.width as f64;
                gui.height = size.height as f64;
            }
            Task::none()
        }
        Message::Focused(window) => {
            app.front = Some(window);
            Task::none()
        }
    }
}

pub fn open_restored(app: &mut App) -> Task<Message> {
    let snapshots = std::mem::take(&mut app.pending_windows);
    let front = app.pending_front.clone();
    let mut tasks = Vec::new();
    if snapshots.is_empty() {
        tasks.push(open_blank(app));
    } else {
        for snapshot in snapshots {
            tasks.push(open_snapshot(app, snapshot));
        }
        if let Some(label) = front
            && let Some(gui) = app.windows.iter().find(|gui| gui.label == label)
        {
            let id = gui.id;
            app.front = Some(id);
            tasks.push(window::gain_focus(id));
        }
    }
    if app.front.is_none() {
        app.front = app.windows.first().map(|gui| gui.id);
    }
    tasks.push(warm_schemas(app));
    tasks.push(warm_lint(app));
    tasks.push(Task::done(Message::ApplyStartup));
    Task::batch(tasks)
}

fn on_tick(app: &mut App) -> Task<Message> {
    app.tick = app.tick.wrapping_add(1);
    let mut tasks = vec![drain_incoming(app)];
    if app.tick % 8 == 0 {
        poll_files(app);
        app.save_session();
    }
    if app.tick % 12 == 0 {
        app.reload_connections();
    }
    if app.tick % 40 == 0 {
        let mut seen = HashSet::new();
        let names: Vec<String> = app
            .windows
            .iter()
            .flat_map(|gui| {
                gui.tabs
                    .iter()
                    .filter_map(|tab| tab.connection_name.clone())
            })
            .filter(|name| seen.insert(name.clone()))
            .collect();
        for name in names {
            tasks.push(ping_connection(app, &name));
        }
    }
    Task::batch(tasks)
}

fn apply_startup(app: &mut App) -> Task<Message> {
    let requests = std::mem::take(&mut app.startup);
    let mut tasks = Vec::new();
    for request in requests {
        tasks.push(insert_request(app, request));
    }
    Task::batch(tasks)
}

fn drain_incoming(app: &mut App) -> Task<Message> {
    let mut tasks = Vec::new();
    loop {
        match app.incoming.try_recv() {
            Ok(Incoming::Request(request)) => tasks.push(insert_request(app, request)),
            Ok(Incoming::Raise) => {
                if let Some(id) = app.front.or_else(|| app.windows.first().map(|gui| gui.id)) {
                    tasks.push(window::gain_focus(id));
                }
            }
            Err(_) => break,
        }
    }
    Task::batch(tasks)
}

fn insert_request(app: &mut App, request: sqlg_core::CliRequest) -> Task<Message> {
    let sqlg_core::CliRequest {
        conn,
        sql,
        target,
        run,
        ..
    } = request;
    let make_window = target == "new-window" || app.windows.is_empty();
    let open_task = if make_window {
        open_blank(app)
    } else {
        Task::none()
    };
    let window_id = if make_window {
        app.windows.last().map(|gui| gui.id)
    } else {
        app.front.or_else(|| app.windows.first().map(|gui| gui.id))
    };
    let Some(window_id) = window_id else {
        return open_task;
    };
    let tab_id = if make_window {
        let gui = app.window_mut(window_id).unwrap();
        let tab = gui.active_tab_mut().unwrap();
        tab.connection_name = Some(conn.clone());
        tab.replace_sql(&sql);
        tab.id.clone()
    } else if let Some(gui) = app.window_mut(window_id) {
        let mut tab = Tab::fresh();
        tab.connection_name = Some(conn.clone());
        tab.replace_sql(&sql);
        let id = tab.id.clone();
        gui.tabs.push(tab);
        gui.active = gui.tabs.len() - 1;
        id
    } else {
        return open_task;
    };
    app.front = Some(window_id);
    let mut tasks = vec![
        open_task,
        window::gain_focus(window_id),
        ensure_schema(app, &conn),
        schedule_lint(app, window_id, &tab_id),
    ];
    if run {
        tasks.push(start_run(app, window_id, &tab_id, false));
    }
    app.save_session();
    Task::batch(tasks)
}

fn on_edit(app: &mut App, window: window::Id, tab: &str, action: Action) -> Task<Message> {
    let edited = action.is_edit();
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Task::none();
        };
        if edited {
            tab_state.remember_undo();
        }
        tab_state.content.perform(action);
    }
    if edited {
        schedule_lint(app, window, tab)
    } else {
        Task::none()
    }
}

fn on_undo(app: &mut App, window: window::Id, tab: &str, undo: bool) -> Task<Message> {
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Task::none();
        };
        let replacement = if undo {
            let Some(previous) = tab_state.undo.pop() else {
                return Task::none();
            };
            tab_state.redo.push(tab_state.sql());
            previous
        } else {
            let Some(next) = tab_state.redo.pop() else {
                return Task::none();
            };
            tab_state.undo.push(tab_state.sql());
            next
        };
        tab_state.content = Content::with_text(&replacement);
    }
    schedule_lint(app, window, tab)
}

fn on_run(
    app: &mut App,
    window: window::Id,
    tab: Option<String>,
    statement: bool,
) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    let tab_id = match tab {
        Some(id) => id,
        None => {
            let Some(id) = active_tab_id(app, window) else {
                return Task::none();
            };
            id
        }
    };
    hide_menu(app, window);
    let run = start_run(app, window, &tab_id, statement);
    let name = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.connection_name.clone());
    let mut tasks = vec![run];
    if let Some(name) = name {
        tasks.push(ensure_schema(app, &name));
    }
    Task::batch(tasks)
}

enum RunPrep {
    Busy,
    NoConnection,
    Empty,
    Ready { sql: String, name: String },
}

fn start_run(app: &mut App, window: window::Id, tab_id: &str, statement: bool) -> Task<Message> {
    let prep = {
        let Some(tab) = app.tab_mut(window, tab_id) else {
            return Task::none();
        };
        if tab.running {
            RunPrep::Busy
        } else if let Some(name) = tab.connection_name.clone() {
            let sql = tab.sql_to_run(statement);
            if sql.trim().is_empty() {
                tab.footer = "Nothing to run".into();
                RunPrep::Empty
            } else {
                tab.results.clear();
                tab.show_rows = logic::VISIBLE_ROWS;
                tab.running = true;
                tab.restored = false;
                tab.footer = "Running…".into();
                RunPrep::Ready { sql, name }
            }
        } else {
            RunPrep::NoConnection
        }
    };
    let (sql, name) = match prep {
        RunPrep::Busy | RunPrep::Empty => return Task::none(),
        RunPrep::NoConnection => {
            app.status = "Choose a connection".into();
            return Task::none();
        }
        RunPrep::Ready { sql, name } => (sql, name),
    };
    let handle = app.db.track_run(&name);
    if let Some(tab) = app.tab_mut(window, tab_id) {
        tab.run_id = handle.run_id.clone();
    }
    let db = app.db.clone();
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let produce_run = handle.run_id.clone();
    let produce_name = name;
    let cancel = handle.cancel;
    let connection_id = handle.connection_id;
    let produce = Task::perform(
        async move {
            let pool = match db.pool_for(&produce_name).await {
                Ok(pool) => pool,
                Err(message) => {
                    let _ = tx.unbounded_send(QueryEvent::StatementError { index: 0, message });
                    let _ = tx.unbounded_send(QueryEvent::Finished {
                        ok: false,
                        executed: 1,
                        total: 1,
                        failed: 1,
                        elapsed_ms: 0,
                        cancelled: false,
                    });
                    db.forget_run(&produce_run);
                    return;
                }
            };
            let tracked = produce_run.clone();
            sqlg_mysql::execute_script(
                pool,
                produce_run.clone(),
                sql,
                cancel,
                connection_id,
                move |event| {
                    let _ = tx.unbounded_send(event);
                },
            )
            .await;
            db.forget_run(&tracked);
        },
        |_| Message::Nop,
    );
    let watched = handle.run_id;
    let tab_key = tab_id.to_string();
    let consume = Task::run(rx, move |event| Message::Query {
        window,
        tab: tab_key.clone(),
        run_id: watched.clone(),
        event,
    });
    Task::batch([produce, consume])
}

fn on_cancel(app: &mut App, window: window::Id, tab: &str) -> Task<Message> {
    let Some(run_id) = app.tab_mut(window, tab).map(|tab| tab.run_id.clone()) else {
        return Task::none();
    };
    if run_id.is_empty() {
        return Task::none();
    }
    let db = app.db.clone();
    Task::perform(
        async move {
            db.cancel_run(&run_id).await;
        },
        |_| Message::Nop,
    )
}

fn on_query(
    app: &mut App,
    window: window::Id,
    tab: &str,
    run_id: &str,
    event: QueryEvent,
) -> Task<Message> {
    let Some(tab_state) = app.tab_mut(window, tab) else {
        return Task::none();
    };
    if tab_state.run_id != run_id {
        return Task::none();
    }
    tab_state.apply_query_event(event);
    Task::none()
}

fn on_new_tab(app: &mut App, window: window::Id) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    let inherited = {
        let Some(gui) = app.window_mut(window) else {
            return Task::none();
        };
        let inherited = gui.active_tab().and_then(|tab| tab.connection_name.clone());
        let mut tab = Tab::fresh();
        tab.connection_name = inherited.clone();
        gui.file_menu = false;
        gui.tabs.push(tab);
        gui.active = gui.tabs.len() - 1;
        inherited
    };
    app.save_session();
    if let Some(name) = inherited {
        ensure_schema(app, &name)
    } else {
        Task::none()
    }
}

fn on_set_connection(app: &mut App, window: window::Id, tab: &str, name: String) -> Task<Message> {
    if let Some(tab_state) = app.tab_mut(window, tab) {
        tab_state.connection_name = Some(name.clone());
        tab_state.results.clear();
        tab_state.footer.clear();
        tab_state.diagnostics.clear();
    }
    app.save_session();
    let mut tasks = vec![ensure_schema(app, &name), schedule_lint(app, window, tab)];
    tasks.push(ping_connection(app, &name));
    Task::batch(tasks)
}

fn on_save(app: &mut App, window: window::Id) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Task::none();
    };
    let selected = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.content.selection())
        .filter(|text| !text.trim().is_empty());
    if let Some(selected) = selected {
        return pick_file(
            window,
            tab_id,
            FilePurpose::SaveSelection { text: selected },
            true,
        );
    }
    let linked = app
        .tab_mut(window, &tab_id)
        .is_some_and(|tab| tab.file_path.is_some());
    if linked {
        let outcome = app
            .tab_mut(window, &tab_id)
            .map(write_linked)
            .unwrap_or(Ok(()));
        return match outcome {
            Ok(()) => {
                app.status = "Saved".into();
                app.save_session();
                Task::none()
            }
            Err(err) => {
                app.status = err;
                Task::none()
            }
        };
    }
    pick_file(window, tab_id, FilePurpose::SaveAs, true)
}

fn on_save_as(app: &mut App, window: window::Id) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Task::none();
    };
    pick_file(window, tab_id, FilePurpose::SaveAs, true)
}

fn on_save_selection(app: &mut App, window: window::Id) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Task::none();
    };
    let selected = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.content.selection())
        .filter(|text| !text.trim().is_empty());
    let Some(selected) = selected else {
        app.status = "Select text to save".into();
        return Task::none();
    };
    pick_file(
        window,
        tab_id,
        FilePurpose::SaveSelection { text: selected },
        true,
    )
}

fn on_file_picked(
    app: &mut App,
    window: window::Id,
    tab: &str,
    purpose: FilePurpose,
    path: Option<PathBuf>,
) -> Task<Message> {
    let Some(path) = path else {
        return Task::none();
    };
    match purpose {
        FilePurpose::Open => open_path(app, window, &path, false),
        FilePurpose::SaveAs => {
            let Some((text, newline, bom)) = app
                .tab_mut(window, tab)
                .map(|tab_state| (tab_state.sql(), tab_state.newline.clone(), tab_state.bom))
            else {
                return Task::none();
            };
            if let Err(err) = files::write_sql_file(&path, &text, &newline, bom) {
                app.status = err;
                return Task::none();
            }
            if let Some(tab_state) = app.tab_mut(window, tab) {
                tab_state.file_path = Some(path.display().to_string());
                tab_state.saved_sql = Some(text);
                tab_state.seen_modified = files::file_modified(&path);
                tab_state.disk_notice = false;
            }
            app.status = "Saved".into();
            app.save_session();
            Task::none()
        }
        FilePurpose::SaveSelection { text } => {
            let newline = app
                .tab_mut(window, tab)
                .map(|tab| tab.newline.clone())
                .unwrap_or_else(|| "\n".into());
            match files::write_sql_file(&path, &text, &newline, false) {
                Ok(()) => app.status = "Saved selection".into(),
                Err(err) => app.status = err,
            }
            Task::none()
        }
    }
}

fn on_drop(app: &mut App, window: window::Id, path: PathBuf) -> Task<Message> {
    open_path(app, window, &path, false)
}

fn open_path(app: &mut App, window: window::Id, path: &Path, force_new: bool) -> Task<Message> {
    let file = match files::read_sql_file(path) {
        Ok(file) => file,
        Err(err) => {
            app.status = err;
            return Task::none();
        }
    };
    let reuse = !force_new
        && app
            .window(window)
            .and_then(|gui| gui.active_tab())
            .is_some_and(|tab| tab.file_path.is_none() && tab.sql().is_empty() && !tab.is_dirty());
    if reuse {
        if let Some(tab) = app.window_mut(window).and_then(|gui| gui.active_tab_mut()) {
            adopt_file(tab, path, &file);
        }
    } else if let Some(gui) = app.window_mut(window) {
        let mut tab = Tab::fresh();
        let inherited = gui
            .active_tab()
            .and_then(|current| current.connection_name.clone());
        tab.connection_name = inherited;
        adopt_file(&mut tab, path, &file);
        gui.tabs.push(tab);
        gui.active = gui.tabs.len() - 1;
        gui.file_menu = false;
    }
    app.save_session();
    let tab_id = active_tab_id(app, window).unwrap_or_default();
    schedule_lint(app, window, &tab_id)
}

fn adopt_file(tab: &mut Tab, path: &Path, file: &TextFile) {
    tab.replace_sql(&file.text);
    tab.file_path = Some(path.display().to_string());
    tab.saved_sql = Some(file.text.clone());
    tab.newline = file.newline.clone();
    tab.bom = file.bom;
    tab.seen_modified = files::file_modified(path);
    tab.disk_notice = false;
    tab.results.clear();
    tab.footer.clear();
}

fn on_reload_disk(app: &mut App, window: window::Id, tab: &str) -> Task<Message> {
    let Some(path) = app
        .tab_mut(window, tab)
        .and_then(|tab| tab.file_path.clone())
    else {
        return Task::none();
    };
    let path = PathBuf::from(path);
    let file = match files::read_sql_file(&path) {
        Ok(file) => file,
        Err(err) => {
            app.status = err;
            return Task::none();
        }
    };
    if let Some(tab_state) = app.tab_mut(window, tab) {
        adopt_file(tab_state, &path, &file);
    }
    schedule_lint(app, window, tab)
}

fn on_lint_ready(
    app: &mut App,
    window: window::Id,
    tab: &str,
    generation: u64,
    diagnostics: Vec<sqlg_core::Diagnostic>,
) -> Task<Message> {
    let Some(sql) = ({
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Task::none();
        };
        if tab_state.lint_generation != generation {
            return Task::none();
        }
        tab_state.diagnostics = diagnostics;
        Some(tab_state.sql())
    }) else {
        return Task::none();
    };
    let Some(name) = app
        .tab_mut(window, tab)
        .and_then(|tab| tab.connection_name.clone())
    else {
        return Task::none();
    };
    let explain = app
        .connection(&name)
        .map(|connection| connection.explain_lint)
        .unwrap_or(false);
    if !explain || !sqlg_mysql::is_single_select(&sql) {
        return Task::none();
    }
    let db = app.db.clone();
    let tab_id = tab.to_string();
    Task::perform(
        async move {
            tokio::time::sleep(Duration::from_millis(750)).await;
            db.explain(&name, &sql).await
        },
        move |result| Message::ExplainReady {
            window,
            tab: tab_id,
            generation,
            result,
        },
    )
}

fn on_explain_ready(
    app: &mut App,
    window: window::Id,
    tab: &str,
    generation: u64,
    result: Result<(), String>,
) -> Task<Message> {
    let Some(tab_state) = app.tab_mut(window, tab) else {
        return Task::none();
    };
    if tab_state.lint_generation != generation {
        return Task::none();
    }
    tab_state
        .diagnostics
        .retain(|diagnostic| diagnostic.severity != "explain");
    if let Err(message) = result {
        tab_state.diagnostics.push(sqlg_core::Diagnostic {
            from: 0,
            to: 0,
            severity: "explain".into(),
            message,
            actions: Vec::new(),
        });
    }
    Task::none()
}

fn on_apply_fix(
    app: &mut App,
    window: window::Id,
    tab: &str,
    from: usize,
    to: usize,
    text: &str,
) -> Task<Message> {
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Task::none();
        };
        let Some(next) = logic::splice_text(&tab_state.sql(), from, to, text) else {
            return Task::none();
        };
        let caret = from + text.len();
        tab_state.remember_undo();
        tab_state.replace_sql(&next);
        let (line, column) = logic::offset_to_position(&next, caret);
        tab_state
            .content
            .move_to(iced::widget::text_editor::Cursor {
                position: iced::widget::text_editor::Position { line, column },
                selection: None,
            });
    }
    schedule_lint(app, window, tab)
}

fn on_drag(app: &mut App, point: Point) -> Task<Message> {
    let planned = {
        let Some(drag) = app.drag.as_mut() else {
            return Task::none();
        };
        let horizontal = !matches!(drag.kind, DragKind::EditorHeight);
        let axis = if horizontal { point.x } else { point.y };
        if drag.origin.is_none() {
            drag.origin = Some(axis);
            return Task::none();
        }
        let delta = axis - drag.origin.unwrap_or(axis);
        let next = match &drag.kind {
            DragKind::Schema => logic::clamp_pane(
                drag.origin_size + delta,
                logic::SCHEMA_MIN,
                logic::SCHEMA_MAX,
            ),
            DragKind::EditorWidth => logic::clamp_pane(
                drag.origin_size + delta,
                logic::EDITOR_MIN,
                logic::EDITOR_MAX,
            ),
            DragKind::EditorHeight => logic::clamp_pane(
                drag.origin_size + delta,
                logic::EDITOR_HEIGHT_MIN,
                logic::EDITOR_HEIGHT_MAX,
            ),
            DragKind::Column { .. } => {
                logic::clamp_pane(drag.origin_size + delta, logic::COLUMN_MIN as f32, 800.0)
            }
        };
        Some((drag.window, drag.kind.clone(), next))
    };
    let Some((window_id, kind, next)) = planned else {
        return Task::none();
    };
    let Some(gui) = app.window_mut(window_id) else {
        return Task::none();
    };
    match kind {
        DragKind::Schema => gui.schema_width = next,
        DragKind::EditorWidth => gui.editor_width = next,
        DragKind::EditorHeight => gui.editor_height = next,
        DragKind::Column { tab, name } => {
            if let Some(tab) = gui.tabs.iter_mut().find(|item| item.id == tab) {
                tab.column_widths.insert(name, next as f64);
            }
        }
    }
    Task::none()
}

fn drag_origin(app: &App, window: window::Id, kind: &DragKind) -> f32 {
    let Some(gui) = app.window(window) else {
        return 200.0;
    };
    match kind {
        DragKind::Schema => gui.schema_width,
        DragKind::EditorWidth => gui.editor_width,
        DragKind::EditorHeight => gui.editor_height,
        DragKind::Column { tab, name } => gui
            .tabs
            .iter()
            .find(|item| &item.id == tab)
            .and_then(|tab| tab.column_widths.get(name))
            .copied()
            .unwrap_or(140.0) as f32,
    }
}

fn request_close_tab(app: &mut App, window: window::Id) -> Task<Message> {
    hide_menu(app, window);
    let needs_prompt = app
        .window(window)
        .and_then(|gui| gui.active_tab())
        .is_some_and(Tab::needs_file_save);
    if needs_prompt {
        let tab_id = active_tab_id(app, window).unwrap_or_default();
        app.modal = Modal::Close(ClosePrompt {
            window,
            tab_ids: vec![tab_id],
            close_window: false,
            quit: false,
        });
        return Task::none();
    }
    close_active_tab(app, window)
}

fn request_close_window(app: &mut App, window: window::Id) -> Task<Message> {
    hide_menu(app, window);
    let dirty = app.dirty_tabs(Some(window));
    if dirty.is_empty() {
        return finish_close_window(app, window);
    }
    app.modal = Modal::Close(ClosePrompt {
        window,
        tab_ids: dirty.into_iter().map(|(_, tab)| tab).collect(),
        close_window: true,
        quit: false,
    });
    Task::none()
}

fn request_quit(app: &mut App) -> Task<Message> {
    if app.blocks_shortcuts() {
        return Task::none();
    }
    let dirty = app.dirty_tabs(None);
    if dirty.is_empty() {
        app.save_session();
        return iced::exit();
    }
    let window = dirty[0].0;
    app.modal = Modal::Close(ClosePrompt {
        window,
        tab_ids: dirty.into_iter().map(|(_, tab)| tab).collect(),
        close_window: false,
        quit: true,
    });
    Task::none()
}

fn apply_close_choice(app: &mut App, save: bool) -> Task<Message> {
    let Modal::Close(prompt) = std::mem::replace(&mut app.modal, Modal::None) else {
        return Task::none();
    };
    if save {
        let targets = if prompt.quit {
            app.dirty_tabs(None)
        } else {
            app.dirty_tabs(Some(prompt.window))
        };
        for (window_id, tab_id) in targets {
            let failed = app.tab_mut(window_id, &tab_id).and_then(|tab| {
                if tab.needs_file_save() {
                    write_linked(tab).err()
                } else {
                    None
                }
            });
            if let Some(err) = failed {
                app.status = err;
                app.modal = Modal::Close(prompt);
                return Task::none();
            }
        }
    }
    if prompt.quit {
        app.save_session();
        return iced::exit();
    }
    if prompt.close_window {
        return finish_close_window(app, prompt.window);
    }
    close_active_tab(app, prompt.window)
}

fn close_active_tab(app: &mut App, window: window::Id) -> Task<Message> {
    let close_window = app
        .window(window)
        .map(|gui| gui.tabs.len() <= 1)
        .unwrap_or(false);
    if close_window {
        return finish_close_window(app, window);
    }
    if let Some(gui) = app.window_mut(window) {
        let index = gui.active.min(gui.tabs.len().saturating_sub(1));
        gui.tabs.remove(index);
        if gui.active >= gui.tabs.len() {
            gui.active = gui.tabs.len().saturating_sub(1);
        }
    }
    app.save_session();
    Task::none()
}

fn finish_close_window(app: &mut App, window: window::Id) -> Task<Message> {
    app.windows.retain(|gui| gui.id != window);
    if app.front == Some(window) {
        app.front = app.windows.last().map(|gui| gui.id);
    }
    app.save_session();
    let close = window::close(window);
    if app.windows.is_empty() {
        Task::batch([close, iced::exit()])
    } else {
        close
    }
}

fn save_form(app: &mut App) -> Task<Message> {
    let Some(draft) = connection_draft(app) else {
        return Task::none();
    };
    let Draft {
        editing,
        connection,
        password,
    } = match draft {
        Ok(draft) => draft,
        Err(err) => {
            set_form_status(app, &err);
            return Task::none();
        }
    };
    let store = app.db.store();
    let saved_name = connection.name.clone();
    let result = if let Some(original) = &editing {
        let patch = ConnectionPatch {
            host: Some(connection.host),
            port: Some(connection.port),
            user: Some(connection.user),
            schema: Some(connection.schema),
            ssl: Some(connection.ssl),
            explain_lint: Some(connection.explain_lint),
            color: Some(connection.color),
            rename: if original == &saved_name {
                None
            } else {
                Some(saved_name.clone())
            },
            password: if password.is_empty() {
                None
            } else {
                Some(password)
            },
        };
        store.update(original, patch).map(|_| ())
    } else {
        store.add(connection, &password)
    };
    match result {
        Ok(()) => {
            if let Some(original) = editing.clone() {
                app.db.forget_pool(&original);
                app.schemas.remove(&original);
            }
            app.db.forget_pool(&saved_name);
            app.schemas.remove(&saved_name);
            app.reload_connections();
            app.status = format!("saved {saved_name}");
            if let Modal::Connections(form) = &mut app.modal {
                form.editing = Some(saved_name);
                form.password.clear();
                form.status = "Saved".into();
            }
        }
        Err(err) => set_form_status(app, &err.to_string()),
    }
    Task::none()
}

fn test_form(app: &mut App) -> Task<Message> {
    let Some(draft) = connection_draft(app) else {
        return Task::none();
    };
    let Draft {
        editing,
        connection,
        password,
    } = match draft {
        Ok(draft) => draft,
        Err(err) => {
            set_form_status(app, &err);
            return Task::none();
        }
    };
    let password = if password.is_empty() {
        editing
            .as_deref()
            .and_then(|name| app.db.store().password(name).ok().flatten())
            .unwrap_or_default()
    } else {
        password
    };
    Task::perform(
        async move { Db::test_connection(&connection, &password).await },
        Message::FormTested,
    )
}

fn delete_form(app: &mut App) -> Task<Message> {
    let Some(name) = (match &app.modal {
        Modal::Connections(form) => form.editing.clone(),
        _ => None,
    }) else {
        return Task::none();
    };
    match app.db.store().remove(&name) {
        Ok(()) => {
            app.db.forget_pool(&name);
            app.schemas.remove(&name);
            app.schema_errors.remove(&name);
            app.pings.remove(&name);
            app.reload_connections();
            app.modal = Modal::Connections(ConnForm::blank());
            app.status = format!("removed {name}");
        }
        Err(err) => set_form_status(app, &err.to_string()),
    }
    Task::none()
}

struct Draft {
    editing: Option<String>,
    connection: Connection,
    password: String,
}

fn connection_draft(app: &App) -> Option<Result<Draft, String>> {
    let Modal::Connections(form) = &app.modal else {
        return None;
    };
    let port = match form.port.trim().parse::<u16>() {
        Ok(port) if port != 0 => port,
        _ => return Some(Err("port must be a number".into())),
    };
    let schema = {
        let trimmed = form.schema.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };
    let color = {
        let trimmed = form.color.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };
    let connection = match (Connection {
        name: form.name.clone(),
        host: form.host.clone(),
        port,
        user: form.user.clone(),
        schema,
        ssl: form.ssl,
        explain_lint: form.explain_lint,
        color,
    })
    .normalized()
    {
        Ok(connection) => connection,
        Err(err) => return Some(Err(err.to_string())),
    };
    Some(Ok(Draft {
        editing: form.editing.clone(),
        password: form.password.clone(),
        connection,
    }))
}

fn set_form_status(app: &mut App, status: &str) {
    if let Modal::Connections(form) = &mut app.modal {
        form.status = status.to_string();
    }
}

fn schedule_lint(app: &mut App, window: window::Id, tab: &str) -> Task<Message> {
    let (generation, sql, connection_name) = {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Task::none();
        };
        tab_state.lint_generation = tab_state.lint_generation.wrapping_add(1);
        (
            tab_state.lint_generation,
            tab_state.sql(),
            tab_state.connection_name.clone(),
        )
    };
    let default_schema = connection_name.as_deref().and_then(|name| {
        app.connection(name)
            .and_then(|connection| connection.schema.clone())
    });
    let snapshot = connection_name
        .as_deref()
        .and_then(|name| app.schemas.get(name));
    let catalog = logic::catalog_for(snapshot, default_schema);
    let tab_id = tab.to_string();
    Task::perform(
        async move {
            tokio::time::sleep(Duration::from_millis(250)).await;
            sqlg_core::lint_sql(&sql, &catalog)
        },
        move |diagnostics| Message::LintReady {
            window,
            tab: tab_id,
            generation,
            diagnostics,
        },
    )
}

fn ensure_schema(app: &mut App, name: &str) -> Task<Message> {
    if name.is_empty() || app.schemas.contains_key(name) || app.schema_loading.contains(name) {
        return Task::none();
    }
    app.schema_loading.insert(name.to_string());
    let db = app.db.clone();
    let owned = name.to_string();
    let label = owned.clone();
    Task::perform(
        async move {
            let pool = db.pool_for(&owned).await?;
            sqlg_mysql::load_schema(&pool).await
        },
        move |result| Message::SchemaLoaded {
            name: label,
            result,
        },
    )
}

fn ping_connection(app: &App, name: &str) -> Task<Message> {
    let db = app.db.clone();
    let owned = name.to_string();
    let label = owned.clone();
    Task::perform(
        async move {
            match db.pool_for(&owned).await {
                Ok(pool) => sqlx::query("select 1").execute(&pool).await.is_ok(),
                Err(_) => false,
            }
        },
        move |ok| Message::Pinged { name: label, ok },
    )
}

fn warm_schemas(app: &mut App) -> Task<Message> {
    let mut seen = HashSet::new();
    let names: Vec<String> = app
        .windows
        .iter()
        .flat_map(|gui| {
            gui.tabs
                .iter()
                .filter_map(|tab| tab.connection_name.clone())
        })
        .filter(|name| seen.insert(name.clone()))
        .collect();
    let mut tasks = Vec::new();
    for name in &names {
        tasks.push(ensure_schema(app, name));
    }
    Task::batch(tasks)
}

fn warm_lint(app: &mut App) -> Task<Message> {
    let ids: Vec<(window::Id, String)> = app
        .windows
        .iter()
        .flat_map(|gui| {
            let id = gui.id;
            gui.tabs.iter().map(move |tab| (id, tab.id.clone()))
        })
        .collect();
    Task::batch(
        ids.into_iter()
            .map(|(window, tab)| schedule_lint(app, window, &tab))
            .collect::<Vec<_>>(),
    )
}

fn poll_files(app: &mut App) {
    let watched: Vec<(window::Id, String, String)> = app
        .windows
        .iter()
        .flat_map(|gui| {
            let id = gui.id;
            gui.tabs
                .iter()
                .filter_map(move |tab| tab.file_path.clone().map(|path| (id, tab.id.clone(), path)))
        })
        .collect();
    for (window, tab, path) in watched {
        let path = PathBuf::from(path);
        let Some(modified) = files::file_modified(&path) else {
            continue;
        };
        let Some(tab_state) = app.tab_mut(window, &tab) else {
            continue;
        };
        if tab_state.seen_modified == Some(modified) {
            continue;
        }
        let Ok(file) = files::read_sql_file(&path) else {
            continue;
        };
        if file.text != tab_state.sql() {
            tab_state.disk_notice = true;
        }
        tab_state.seen_modified = Some(modified);
    }
}

fn write_linked(tab: &mut Tab) -> Result<(), String> {
    let Some(path) = tab.file_path.clone() else {
        return Err("no file".into());
    };
    let text = tab.sql();
    files::write_sql_file(Path::new(&path), &text, &tab.newline, tab.bom)?;
    tab.saved_sql = Some(text);
    tab.seen_modified = files::file_modified(Path::new(&path));
    tab.disk_notice = false;
    Ok(())
}

fn pick_file(window: window::Id, tab: String, purpose: FilePurpose, save: bool) -> Task<Message> {
    Task::perform(
        async move {
            let dialog = rfd::AsyncFileDialog::new().add_filter("SQL", &["sql"]);
            let handle = if save {
                dialog.save_file().await
            } else {
                dialog.pick_file().await
            };
            handle.map(|file| file.path().to_path_buf())
        },
        move |path| Message::FilePicked {
            window,
            tab,
            purpose,
            path,
        },
    )
}

fn open_blank(app: &mut App) -> Task<Message> {
    let (id, task) = window::open(window_settings(None));
    app.windows.push(GuiWindow::fresh(id));
    app.front = Some(id);
    task.map(|_| Message::Nop)
}

fn open_snapshot(app: &mut App, snapshot: WindowSnapshot) -> Task<Message> {
    let (id, task) = window::open(window_settings(Some(&snapshot)));
    app.windows.push(GuiWindow::from_snapshot(id, snapshot));
    if app.front.is_none() {
        app.front = Some(id);
    }
    task.map(|_| Message::Nop)
}

fn window_settings(snapshot: Option<&WindowSnapshot>) -> window::Settings {
    let mut settings = window::Settings {
        size: Size::new(1200.0, 800.0),
        min_size: Some(Size::new(720.0, 480.0)),
        position: window::Position::Centered,
        exit_on_close_request: false,
        ..window::Settings::default()
    };
    if let Some(snapshot) = snapshot {
        settings.size = Size::new(
            snapshot.width.max(720.0) as f32,
            snapshot.height.max(480.0) as f32,
        );
        settings.position =
            window::Position::Specific(Point::new(snapshot.x as f32, snapshot.y as f32));
        settings.fullscreen = snapshot.fullscreen;
    }
    settings
}

fn active_tab_id(app: &App, window: window::Id) -> Option<String> {
    app.window(window)
        .and_then(|gui| gui.active_tab())
        .map(|tab| tab.id.clone())
}

fn hide_menu(app: &mut App, window: window::Id) {
    if let Some(gui) = app.window_mut(window) {
        gui.file_menu = false;
    }
}
