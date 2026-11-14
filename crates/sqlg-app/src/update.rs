use sqlg_core::{Connection, ConnectionPatch};
use sqlg_mysql::QueryEvent;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::files::{self, TextFile};
use crate::ipc::Incoming;
use crate::logic;
use crate::message::{
    DragKind, Effect, ExplainJob, Field, FilePurpose, LintJob, Message, QueryJob,
};
use crate::model::{App, ClosePrompt, ConnForm, Drag, GuiWindow, Modal, Tab, ordered};

pub fn startup(app: &mut App) -> Vec<Effect> {
    let mut effects: Vec<Effect> = app
        .windows
        .iter()
        .filter(|gui| gui.id != app.root_id)
        .map(|gui| Effect::OpenWindow(gui.id))
        .collect();
    if let Some(id) = app.front {
        effects.push(Effect::Focus(id));
    }
    effects.extend(warm_schemas(app));
    effects.extend(warm_lint(app));
    effects.extend(apply_startup(app));
    effects
}

pub fn update(app: &mut App, message: Message) -> Vec<Effect> {
    match message {
        Message::Nop => Vec::new(),
        Message::Tick => on_tick(app),
        Message::Edit { window, tab, text } => on_edit(app, window, &tab, text),
        Message::Selection {
            window,
            tab,
            anchor,
            head,
        } => {
            if let Some(tab) = app.tab_mut(window, &tab) {
                tab.buffer.set_utf16_span(anchor, head);
            }
            Vec::new()
        }
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
                return Vec::new();
            }
            open_blank(app)
        }
        Message::CloseTab(window) => {
            if app.blocks_shortcuts() {
                return Vec::new();
            }
            request_close_tab(app, window)
        }
        Message::CloseWindow(window) => {
            if app.blocks_shortcuts() {
                return Vec::new();
            }
            request_close_window(app, window)
        }
        Message::SelectTab { window, index } => {
            if let Some(gui) = app.window_mut(window)
                && index < gui.tabs.len()
            {
                gui.active = index;
                gui.file_menu = false;
            }
            Vec::new()
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
            Vec::new()
        }
        Message::SetTheme(name) => {
            let theme = crate::theme::find_theme(&name);
            if theme.id == name {
                app.theme_name = theme.id.into();
                app.save_session();
            }
            Vec::new()
        }
        Message::ToggleFileMenu(window) => {
            if let Some(gui) = app.window_mut(window) {
                gui.file_menu = !gui.file_menu;
            }
            Vec::new()
        }
        Message::Open(window) => {
            if app.blocks_shortcuts() {
                return Vec::new();
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
            Vec::new()
        }
        Message::InsertText { window, tab, text } => {
            if let Some(tab_state) = app.tab_mut(window, &tab) {
                insert_at_cursor(tab_state, &text);
            }
            schedule_lint(app, window, &tab)
        }
        Message::ToggleExpand { window, key } => {
            if let Some(gui) = app.window_mut(window)
                && !gui.expanded.insert(key.clone())
            {
                gui.expanded.remove(&key);
            }
            Vec::new()
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
            Vec::new()
        }
        Message::Pinged { name, ok } => {
            app.pings.insert(name, ok);
            Vec::new()
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
            Vec::new()
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
        Message::Copied => {
            app.status = "Copied".into();
            Vec::new()
        }
        Message::ShowMore { window, tab } => {
            if let Some(tab) = app.tab_mut(window, &tab) {
                tab.show_rows = tab.show_rows.saturating_add(logic::VISIBLE_ROWS);
            }
            Vec::new()
        }
        Message::OpenConnections(window) => {
            if app.blocks_shortcuts() {
                return Vec::new();
            }
            hide_menu(app, window);
            app.show_connections(ConnForm::blank());
            Vec::new()
        }
        Message::FormNew => {
            app.show_connections(ConnForm::blank());
            Vec::new()
        }
        Message::FormPick(name) => {
            let form = app
                .connection(&name)
                .map(ConnForm::from_connection)
                .unwrap_or_else(ConnForm::blank);
            app.show_connections(form);
            Vec::new()
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
            Vec::new()
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
            Vec::new()
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
            Vec::new()
        }
        Message::DismissModal => {
            app.modal = Modal::None;
            Vec::new()
        }
        Message::CloseChoice { save } => apply_close_choice(app, save),
        Message::InstallCli => vec![Effect::Install],
        Message::UninstallCli => vec![Effect::Uninstall],
        Message::CliResult(result) => {
            app.status = match result {
                Ok(text) => text,
                Err(err) => err,
            };
            Vec::new()
        }
        Message::StartDrag { window, kind, x, y } => {
            let origin_size = drag_origin(app, window, &kind);
            let horizontal = !matches!(kind, DragKind::EditorHeight);
            app.drag = Some(Drag {
                window,
                kind,
                origin: if horizontal { x } else { y },
                origin_size,
                horizontal,
            });
            Vec::new()
        }
        Message::DragMove { x, y } => on_drag(app, x, y),
        Message::DragEnd => {
            if app.drag.take().is_some() {
                app.save_session();
            }
            Vec::new()
        }
        Message::Quit => request_quit(app),
        Message::Escape(window) => {
            if !matches!(app.modal, Modal::None) {
                app.modal = Modal::None;
            } else if let Some(gui) = app.window_mut(window) {
                gui.file_menu = false;
            }
            Vec::new()
        }
        Message::Moved { window, x, y } => {
            if let Some(gui) = app.window_mut(window) {
                gui.x = x;
                gui.y = y;
            }
            Vec::new()
        }
        Message::Resized {
            window,
            width,
            height,
        } => {
            if let Some(gui) = app.window_mut(window) {
                gui.width = width;
                gui.height = height;
            }
            Vec::new()
        }
        Message::Focused(window) => {
            app.front = Some(window);
            Vec::new()
        }
    }
}

fn on_tick(app: &mut App) -> Vec<Effect> {
    app.tick = app.tick.wrapping_add(1);
    let mut effects = vec![drain_incoming(app)];
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
            effects.push(vec![ping_connection(&name)]);
        }
    }
    effects.into_iter().flatten().collect()
}

fn apply_startup(app: &mut App) -> Vec<Effect> {
    let requests = std::mem::take(&mut app.startup);
    let mut effects = Vec::new();
    for request in requests {
        effects.extend(insert_request(app, request));
    }
    effects
}

fn drain_incoming(app: &mut App) -> Vec<Effect> {
    let mut effects = Vec::new();
    loop {
        match app.incoming.try_recv() {
            Ok(Incoming::Request(request)) => effects.extend(insert_request(app, request)),
            Ok(Incoming::Raise) => {
                if let Some(id) = app.front.or_else(|| app.windows.first().map(|gui| gui.id)) {
                    effects.push(Effect::Focus(id));
                }
            }
            Err(_) => break,
        }
    }
    effects
}

fn insert_request(app: &mut App, request: sqlg_core::CliRequest) -> Vec<Effect> {
    let sqlg_core::CliRequest {
        conn,
        sql,
        target,
        run,
        ..
    } = request;
    let make_window = target == "new-window" || app.windows.is_empty();
    let open_effects = if make_window {
        open_blank(app)
    } else {
        Vec::new()
    };
    let window_id = if make_window {
        app.windows.last().map(|gui| gui.id)
    } else {
        app.front.or_else(|| app.windows.first().map(|gui| gui.id))
    };
    let Some(window_id) = window_id else {
        return open_effects;
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
        return open_effects;
    };
    app.front = Some(window_id);
    let mut effects = open_effects;
    effects.push(Effect::Focus(window_id));
    effects.extend(ensure_schema(app, &conn));
    effects.extend(schedule_lint(app, window_id, &tab_id));
    if run {
        effects.extend(start_run(app, window_id, &tab_id, false));
    }
    app.save_session();
    effects
}

fn on_edit(app: &mut App, window: u64, tab: &str, text: String) -> Vec<Effect> {
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Vec::new();
        };
        if tab_state.buffer.text == text {
            return Vec::new();
        }
        tab_state.remember_undo();
        tab_state.buffer.set_typed(text);
        tab_state.restored = false;
    }
    schedule_lint(app, window, tab)
}

fn on_undo(app: &mut App, window: u64, tab: &str, undo: bool) -> Vec<Effect> {
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Vec::new();
        };
        let replacement = if undo {
            let Some(previous) = tab_state.undo.pop() else {
                return Vec::new();
            };
            tab_state.redo.push(tab_state.sql());
            previous
        } else {
            let Some(next) = tab_state.redo.pop() else {
                return Vec::new();
            };
            tab_state.undo.push(tab_state.sql());
            next
        };
        tab_state.buffer.replace_text(&replacement);
        tab_state.restored = false;
    }
    schedule_lint(app, window, tab)
}

fn on_run(app: &mut App, window: u64, tab: Option<String>, statement: bool) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    let tab_id = match tab {
        Some(id) => id,
        None => {
            let Some(id) = active_tab_id(app, window) else {
                return Vec::new();
            };
            id
        }
    };
    hide_menu(app, window);
    let run = start_run(app, window, &tab_id, statement);
    let name = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.connection_name.clone());
    let mut effects = run;
    if let Some(name) = name {
        effects.extend(ensure_schema(app, &name));
    }
    effects
}

enum RunPrep {
    Busy,
    NoConnection,
    Empty,
    Ready { sql: String, name: String },
}

fn start_run(app: &mut App, window: u64, tab_id: &str, statement: bool) -> Vec<Effect> {
    let prep = {
        let Some(tab) = app.tab_mut(window, tab_id) else {
            return Vec::new();
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
        RunPrep::Busy | RunPrep::Empty => return Vec::new(),
        RunPrep::NoConnection => {
            app.status = "Choose a connection".into();
            return Vec::new();
        }
        RunPrep::Ready { sql, name } => (sql, name),
    };
    let handle = app.db.track_run(&name);
    if let Some(tab) = app.tab_mut(window, tab_id) {
        tab.run_id = handle.run_id.clone();
    }
    vec![Effect::Query(QueryJob {
        window,
        tab: tab_id.to_string(),
        run_id: handle.run_id,
        sql,
        connection: name,
        cancel: handle.cancel,
        connection_id: handle.connection_id,
    })]
}

fn on_cancel(app: &mut App, window: u64, tab: &str) -> Vec<Effect> {
    let Some(run_id) = app.tab_mut(window, tab).map(|tab| tab.run_id.clone()) else {
        return Vec::new();
    };
    if run_id.is_empty() {
        return Vec::new();
    }
    vec![Effect::Cancel(run_id)]
}

fn on_query(app: &mut App, window: u64, tab: &str, run_id: &str, event: QueryEvent) -> Vec<Effect> {
    let Some(tab_state) = app.tab_mut(window, tab) else {
        return Vec::new();
    };
    if tab_state.run_id != run_id {
        return Vec::new();
    }
    tab_state.apply_query_event(event);
    Vec::new()
}

fn on_new_tab(app: &mut App, window: u64) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    let inherited = {
        let Some(gui) = app.window_mut(window) else {
            return Vec::new();
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
        Vec::new()
    }
}

fn on_set_connection(app: &mut App, window: u64, tab: &str, name: String) -> Vec<Effect> {
    if name.is_empty() {
        if let Some(tab_state) = app.tab_mut(window, tab) {
            tab_state.connection_name = None;
            tab_state.results.clear();
            tab_state.footer.clear();
            tab_state.diagnostics.clear();
        }
        app.save_session();
        return Vec::new();
    }
    if let Some(tab_state) = app.tab_mut(window, tab) {
        tab_state.connection_name = Some(name.clone());
        tab_state.results.clear();
        tab_state.footer.clear();
        tab_state.diagnostics.clear();
    }
    app.save_session();
    let mut effects = ensure_schema(app, &name);
    effects.extend(schedule_lint(app, window, tab));
    effects.push(ping_connection(&name));
    effects
}

fn on_save(app: &mut App, window: u64) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Vec::new();
    };
    let selected = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.selected_text())
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
                Vec::new()
            }
            Err(err) => {
                app.status = err;
                Vec::new()
            }
        };
    }
    pick_file(window, tab_id, FilePurpose::SaveAs, true)
}

fn on_save_as(app: &mut App, window: u64) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Vec::new();
    };
    pick_file(window, tab_id, FilePurpose::SaveAs, true)
}

fn on_save_selection(app: &mut App, window: u64) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    hide_menu(app, window);
    let Some(tab_id) = active_tab_id(app, window) else {
        return Vec::new();
    };
    let selected = app
        .tab_mut(window, &tab_id)
        .and_then(|tab| tab.selected_text())
        .filter(|text| !text.trim().is_empty());
    let Some(selected) = selected else {
        app.status = "Select text to save".into();
        return Vec::new();
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
    window: u64,
    tab: &str,
    purpose: FilePurpose,
    path: Option<PathBuf>,
) -> Vec<Effect> {
    let Some(path) = path else {
        return Vec::new();
    };
    match purpose {
        FilePurpose::Open => open_path(app, window, &path, false),
        FilePurpose::SaveAs => {
            let Some((text, newline, bom)) = app
                .tab_mut(window, tab)
                .map(|tab_state| (tab_state.sql(), tab_state.newline.clone(), tab_state.bom))
            else {
                return Vec::new();
            };
            if let Err(err) = files::write_sql_file(&path, &text, &newline, bom) {
                app.status = err;
                return Vec::new();
            }
            if let Some(tab_state) = app.tab_mut(window, tab) {
                tab_state.file_path = Some(path.display().to_string());
                tab_state.saved_sql = Some(text);
                tab_state.seen_modified = files::file_modified(&path);
                tab_state.disk_notice = false;
            }
            app.status = "Saved".into();
            app.save_session();
            Vec::new()
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
            Vec::new()
        }
    }
}

fn on_drop(app: &mut App, window: u64, path: PathBuf) -> Vec<Effect> {
    open_path(app, window, &path, false)
}

fn open_path(app: &mut App, window: u64, path: &Path, force_new: bool) -> Vec<Effect> {
    let file = match files::read_sql_file(path) {
        Ok(file) => file,
        Err(err) => {
            app.status = err;
            return Vec::new();
        }
    };
    let display = path.display().to_string();
    let existing = app.window(window).and_then(|gui| {
        gui.tabs
            .iter()
            .position(|tab| tab.file_path.as_deref() == Some(display.as_str()))
            .map(|index| (index, gui.tabs[index].id.clone()))
    });
    if let Some((index, tab_id)) = existing {
        if let Some(gui) = app.window_mut(window) {
            gui.active = index;
            gui.file_menu = false;
        }
        app.save_session();
        return schedule_lint(app, window, &tab_id);
    }
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

fn on_reload_disk(app: &mut App, window: u64, tab: &str) -> Vec<Effect> {
    let Some(path) = app
        .tab_mut(window, tab)
        .and_then(|tab| tab.file_path.clone())
    else {
        return Vec::new();
    };
    let path = PathBuf::from(path);
    let file = match files::read_sql_file(&path) {
        Ok(file) => file,
        Err(err) => {
            app.status = err;
            return Vec::new();
        }
    };
    if let Some(tab_state) = app.tab_mut(window, tab) {
        adopt_file(tab_state, &path, &file);
    }
    schedule_lint(app, window, tab)
}

fn on_lint_ready(
    app: &mut App,
    window: u64,
    tab: &str,
    generation: u64,
    diagnostics: Vec<sqlg_core::Diagnostic>,
) -> Vec<Effect> {
    let Some(sql) = ({
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Vec::new();
        };
        if tab_state.lint_generation != generation {
            return Vec::new();
        }
        tab_state.diagnostics = diagnostics;
        Some(tab_state.sql())
    }) else {
        return Vec::new();
    };
    let Some(name) = app
        .tab_mut(window, tab)
        .and_then(|tab| tab.connection_name.clone())
    else {
        return Vec::new();
    };
    let explain = app
        .connection(&name)
        .map(|connection| connection.explain_lint)
        .unwrap_or(false);
    if !explain || !sqlg_mysql::is_single_select(&sql) {
        return Vec::new();
    }
    vec![Effect::Explain(ExplainJob {
        window,
        tab: tab.to_string(),
        generation,
        name,
        sql,
    })]
}

fn on_explain_ready(
    app: &mut App,
    window: u64,
    tab: &str,
    generation: u64,
    result: Result<(), String>,
) -> Vec<Effect> {
    let Some(tab_state) = app.tab_mut(window, tab) else {
        return Vec::new();
    };
    if tab_state.lint_generation != generation {
        return Vec::new();
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
    Vec::new()
}

fn on_apply_fix(
    app: &mut App,
    window: u64,
    tab: &str,
    from: usize,
    to: usize,
    text: &str,
) -> Vec<Effect> {
    {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Vec::new();
        };
        let Some(next) = logic::splice_text(&tab_state.sql(), from, to, text) else {
            return Vec::new();
        };
        let caret = from + text.len();
        tab_state.remember_undo();
        tab_state.buffer.replace_text(&next);
        tab_state.buffer.anchor = caret.min(tab_state.buffer.text.len());
        tab_state.buffer.head = tab_state.buffer.anchor;
    }
    schedule_lint(app, window, tab)
}

fn on_drag(app: &mut App, x: f32, y: f32) -> Vec<Effect> {
    let planned = {
        let Some(drag) = app.drag.as_ref() else {
            return Vec::new();
        };
        let axis = if drag.horizontal { x } else { y };
        let delta = axis - drag.origin;
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
        return Vec::new();
    };
    let Some(gui) = app.window_mut(window_id) else {
        return Vec::new();
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
    Vec::new()
}

fn drag_origin(app: &App, window: u64, kind: &DragKind) -> f32 {
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

fn request_close_tab(app: &mut App, window: u64) -> Vec<Effect> {
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
        return Vec::new();
    }
    close_active_tab(app, window)
}

fn request_close_window(app: &mut App, window: u64) -> Vec<Effect> {
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
    Vec::new()
}

fn request_quit(app: &mut App) -> Vec<Effect> {
    if app.blocks_shortcuts() {
        return Vec::new();
    }
    let dirty = app.dirty_tabs(None);
    if dirty.is_empty() {
        app.save_session();
        return vec![Effect::Exit];
    }
    let window = dirty[0].0;
    app.modal = Modal::Close(ClosePrompt {
        window,
        tab_ids: dirty.into_iter().map(|(_, tab)| tab).collect(),
        close_window: false,
        quit: true,
    });
    Vec::new()
}

fn apply_close_choice(app: &mut App, save: bool) -> Vec<Effect> {
    let Modal::Close(prompt) = std::mem::replace(&mut app.modal, Modal::None) else {
        return Vec::new();
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
                return Vec::new();
            }
        }
    }
    if prompt.quit {
        app.save_session();
        return vec![Effect::Exit];
    }
    if prompt.close_window {
        return finish_close_window(app, prompt.window);
    }
    close_active_tab(app, prompt.window)
}

fn close_active_tab(app: &mut App, window: u64) -> Vec<Effect> {
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
    Vec::new()
}

fn finish_close_window(app: &mut App, window: u64) -> Vec<Effect> {
    app.windows.retain(|gui| gui.id != window);
    if app.front == Some(window) {
        app.front = app.windows.last().map(|gui| gui.id);
    }
    app.save_session();
    if app.windows.is_empty() {
        vec![Effect::Close(window), Effect::Exit]
    } else {
        vec![Effect::Close(window)]
    }
}

fn save_form(app: &mut App) -> Vec<Effect> {
    let Some(draft) = connection_draft(app) else {
        return Vec::new();
    };
    let Draft {
        editing,
        connection,
        password,
    } = match draft {
        Ok(draft) => draft,
        Err(err) => {
            set_form_status(app, &err);
            return Vec::new();
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
            let mut form = ConnForm::blank();
            if let Modal::Connections(current) = &app.modal {
                form = ConnForm {
                    editing: Some(saved_name.clone()),
                    name: current.name.clone(),
                    host: current.host.clone(),
                    port: current.port.clone(),
                    user: current.user.clone(),
                    password: String::new(),
                    schema: current.schema.clone(),
                    color: current.color.clone(),
                    ssl: current.ssl,
                    explain_lint: current.explain_lint,
                    status: "Saved".into(),
                };
            }
            app.show_connections(form);
        }
        Err(err) => set_form_status(app, &err.to_string()),
    }
    Vec::new()
}

fn test_form(app: &mut App) -> Vec<Effect> {
    let Some(draft) = connection_draft(app) else {
        return Vec::new();
    };
    let Draft {
        editing,
        connection,
        password,
    } = match draft {
        Ok(draft) => draft,
        Err(err) => {
            set_form_status(app, &err);
            return Vec::new();
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
    vec![Effect::Test {
        connection,
        password,
    }]
}

fn delete_form(app: &mut App) -> Vec<Effect> {
    let Some(name) = (match &app.modal {
        Modal::Connections(form) => form.editing.clone(),
        _ => None,
    }) else {
        return Vec::new();
    };
    match app.db.store().remove(&name) {
        Ok(()) => {
            app.db.forget_pool(&name);
            app.schemas.remove(&name);
            app.schema_errors.remove(&name);
            app.pings.remove(&name);
            app.reload_connections();
            app.show_connections(ConnForm::blank());
            app.status = format!("removed {name}");
        }
        Err(err) => set_form_status(app, &err.to_string()),
    }
    Vec::new()
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

fn schedule_lint(app: &mut App, window: u64, tab: &str) -> Vec<Effect> {
    let (generation, sql, connection_name) = {
        let Some(tab_state) = app.tab_mut(window, tab) else {
            return Vec::new();
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
    vec![Effect::Lint(LintJob {
        window,
        tab: tab.to_string(),
        generation,
        sql,
        catalog,
    })]
}

fn ensure_schema(app: &mut App, name: &str) -> Vec<Effect> {
    if name.is_empty() || app.schemas.contains_key(name) || app.schema_loading.contains(name) {
        return Vec::new();
    }
    app.schema_loading.insert(name.to_string());
    vec![Effect::Schema(name.to_string())]
}

fn ping_connection(name: &str) -> Effect {
    Effect::Ping(name.to_string())
}

fn warm_schemas(app: &mut App) -> Vec<Effect> {
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
    let mut effects = Vec::new();
    for name in &names {
        effects.extend(ensure_schema(app, name));
    }
    effects
}

fn warm_lint(app: &mut App) -> Vec<Effect> {
    let ids: Vec<(u64, String)> = app
        .windows
        .iter()
        .flat_map(|gui| {
            let id = gui.id;
            gui.tabs.iter().map(move |tab| (id, tab.id.clone()))
        })
        .collect();
    let mut effects = Vec::new();
    for (window, tab) in ids {
        effects.extend(schedule_lint(app, window, &tab));
    }
    effects
}

fn poll_files(app: &mut App) {
    let watched: Vec<(u64, String, String)> = app
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

fn pick_file(window: u64, tab: String, purpose: FilePurpose, save: bool) -> Vec<Effect> {
    vec![Effect::Pick {
        window,
        tab,
        purpose,
        save,
    }]
}

fn open_blank(app: &mut App) -> Vec<Effect> {
    let id = app.alloc_id();
    let first = app.windows.is_empty();
    app.windows.push(GuiWindow::fresh(id));
    app.front = Some(id);
    if first {
        app.root_id = id;
        Vec::new()
    } else {
        vec![Effect::OpenWindow(id)]
    }
}

fn insert_at_cursor(tab: &mut Tab, text: &str) {
    tab.remember_undo();
    let (start, end) = ordered(tab.buffer.anchor, tab.buffer.head);
    let Some(next) = logic::splice_text(&tab.sql(), start, end, text) else {
        return;
    };
    let caret = start + text.len();
    tab.buffer.replace_text(&next);
    let caret = caret.min(tab.buffer.text.len());
    tab.buffer.anchor = caret;
    tab.buffer.head = caret;
}

fn active_tab_id(app: &App, window: u64) -> Option<String> {
    app.window(window)
        .and_then(|gui| gui.active_tab())
        .map(|tab| tab.id.clone())
}

fn hide_menu(app: &mut App, window: u64) {
    if let Some(gui) = app.window_mut(window) {
        gui.file_menu = false;
    }
}
