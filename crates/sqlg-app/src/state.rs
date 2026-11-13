use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use iced::widget::text_editor::{Content, Cursor, Position};
use iced::window;
use sqlg_core::{Connection, SessionDocument, TabSnapshot, WindowSnapshot, statement_at};
use sqlg_mysql::{ColumnInfo, QueryEvent, SchemaSnapshot};

use crate::db::Db;
use crate::ipc::{Incoming, Primary};
use crate::logic::{self, is_dirty, needs_file_save};
use crate::message::DragKind;

pub struct App {
    pub windows: Vec<GuiWindow>,
    pub front: Option<window::Id>,
    pub theme_name: String,
    pub connections: Vec<Connection>,
    pub connection_names: Vec<String>,
    pub schemas: HashMap<String, SchemaSnapshot>,
    pub schema_errors: HashMap<String, String>,
    pub schema_loading: HashSet<String>,
    pub pings: HashMap<String, bool>,
    pub db: Arc<Db>,
    pub modal: Modal,
    pub drag: Option<Drag>,
    pub status: String,
    pub data_dir: PathBuf,
    pub tick: u64,
    pub startup: Vec<sqlg_core::CliRequest>,
    pub pending_windows: Vec<WindowSnapshot>,
    pub pending_front: Option<String>,
    pub incoming: Receiver<Incoming>,
    pub _lock: std::fs::File,
}

pub struct GuiWindow {
    pub id: window::Id,
    pub label: String,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub schema_width: f32,
    pub editor_width: f32,
    pub editor_height: f32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub fullscreen: bool,
    pub file_menu: bool,
    pub expanded: HashSet<String>,
}

pub struct Tab {
    pub id: String,
    pub connection_name: Option<String>,
    pub content: Content,
    pub layout: String,
    pub column_widths: BTreeMap<String, f64>,
    pub file_path: Option<String>,
    pub saved_sql: Option<String>,
    pub newline: String,
    pub bom: bool,
    pub seen_modified: Option<SystemTime>,
    pub results: Vec<ResultSection>,
    pub show_rows: usize,
    pub running: bool,
    pub run_id: String,
    pub footer: String,
    pub restored: bool,
    pub disk_notice: bool,
    pub diagnostics: Vec<sqlg_core::Diagnostic>,
    pub lint_generation: u64,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}

pub struct ResultSection {
    pub sql: String,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<Option<String>>>,
    pub error: Option<String>,
    pub rows_affected: Option<u64>,
    pub elapsed_ms: Option<u64>,
    pub truncated: bool,
}

pub enum Modal {
    None,
    Connections(ConnForm),
    Close(ClosePrompt),
}

pub struct ConnForm {
    pub editing: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub schema: String,
    pub color: String,
    pub ssl: bool,
    pub explain_lint: bool,
    pub status: String,
}

pub struct ClosePrompt {
    pub window: window::Id,
    pub tab_ids: Vec<String>,
    pub close_window: bool,
    pub quit: bool,
}

pub struct Drag {
    pub window: window::Id,
    pub kind: DragKind,
    pub origin: Option<f32>,
    pub origin_size: f32,
}

impl App {
    pub fn boot(primary: Primary) -> Self {
        let data_dir = primary_dir(&primary);
        let session = load_session(&data_dir);
        let db = Arc::new(Db::new(data_dir.clone()));
        let mut app = Self {
            windows: Vec::new(),
            front: None,
            theme_name: if session.theme == "dark" {
                "dark".into()
            } else {
                "light".into()
            },
            connections: Vec::new(),
            connection_names: Vec::new(),
            schemas: HashMap::new(),
            schema_errors: HashMap::new(),
            schema_loading: HashSet::new(),
            pings: HashMap::new(),
            db,
            modal: Modal::None,
            drag: None,
            status: String::new(),
            data_dir,
            tick: 0,
            startup: primary.startup,
            pending_windows: session.windows,
            pending_front: session.front_window,
            incoming: primary.incoming,
            _lock: primary._lock,
        };
        app.reload_connections();
        app
    }

    pub fn window(&self, id: window::Id) -> Option<&GuiWindow> {
        self.windows.iter().find(|window| window.id == id)
    }

    pub fn window_mut(&mut self, id: window::Id) -> Option<&mut GuiWindow> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    pub fn tab_mut(&mut self, id: window::Id, tab: &str) -> Option<&mut Tab> {
        self.window_mut(id)?
            .tabs
            .iter_mut()
            .find(|item| item.id == tab)
    }

    pub fn connection(&self, name: &str) -> Option<&Connection> {
        self.connections
            .iter()
            .find(|connection| connection.name == name)
    }

    pub fn blocks_shortcuts(&self) -> bool {
        !matches!(self.modal, Modal::None)
    }

    pub fn reload_connections(&mut self) {
        let listed = self.db.store().list().unwrap_or_default();
        let mut changed = Vec::new();
        for old in &self.connections {
            match listed.iter().find(|item| item.name == old.name) {
                Some(new) if new != old => changed.push(old.name.clone()),
                None => changed.push(old.name.clone()),
                _ => {}
            }
        }
        self.connections = listed;
        self.connection_names = self
            .connections
            .iter()
            .map(|connection| connection.name.clone())
            .collect();
        for name in changed {
            self.schemas.remove(&name);
            self.schema_errors.remove(&name);
            self.schema_loading.remove(&name);
            self.pings.remove(&name);
            self.db.forget_pool(&name);
        }
    }

    pub fn snapshot(&self) -> SessionDocument {
        let mut windows: Vec<WindowSnapshot> = self
            .windows
            .iter()
            .enumerate()
            .map(|(index, window)| window.snapshot(index as u32))
            .collect();
        windows.sort_by_key(|window| window.order);
        let front_window = self
            .front
            .and_then(|id| self.window(id))
            .map(|window| window.label.clone())
            .or_else(|| windows.first().map(|window| window.label.clone()));
        SessionDocument {
            version: 1,
            theme: self.theme_name.clone(),
            front_window,
            windows,
        }
    }

    pub fn save_session(&self) {
        let path = self.data_dir.join("session.json");
        let Ok(bytes) = serde_json::to_vec_pretty(&self.snapshot()) else {
            return;
        };
        let _ = sqlg_core::atomic_write(&path, &bytes);
    }

    pub fn dirty_tabs(&self, window: Option<window::Id>) -> Vec<(window::Id, String)> {
        self.windows
            .iter()
            .filter(|item| window.is_none_or(|id| item.id == id))
            .flat_map(|item| {
                item.tabs.iter().filter_map(move |tab| {
                    if tab.needs_file_save() {
                        Some((item.id, tab.id.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}

impl GuiWindow {
    pub fn fresh(id: window::Id) -> Self {
        Self::with_tabs(id, vec![Tab::fresh()])
    }

    pub fn with_tabs(id: window::Id, tabs: Vec<Tab>) -> Self {
        Self {
            id,
            label: format!("win-{}", uuid::Uuid::new_v4()),
            tabs,
            active: 0,
            schema_width: 250.0,
            editor_width: 520.0,
            editor_height: 320.0,
            x: 80.0,
            y: 80.0,
            width: 1200.0,
            height: 800.0,
            fullscreen: false,
            file_menu: false,
            expanded: HashSet::new(),
        }
    }

    pub fn from_snapshot(id: window::Id, snapshot: WindowSnapshot) -> Self {
        let mut tabs: Vec<Tab> = snapshot.tabs.into_iter().map(Tab::from_snapshot).collect();
        if tabs.is_empty() {
            tabs.push(Tab::fresh());
        }
        let active = tabs
            .iter()
            .position(|tab| tab.id == snapshot.active_tab)
            .unwrap_or(0);
        Self {
            id,
            label: snapshot.label,
            tabs,
            active,
            schema_width: logic::pane_size(snapshot.schema_width, 250.0, logic::SCHEMA_MIN),
            editor_width: logic::pane_size(snapshot.editor_width, 520.0, logic::EDITOR_MIN),
            editor_height: logic::pane_size(
                snapshot.editor_height,
                320.0,
                logic::EDITOR_HEIGHT_MIN,
            ),
            x: snapshot.x,
            y: snapshot.y,
            width: snapshot.width.max(720.0),
            height: snapshot.height.max(480.0),
            fullscreen: snapshot.fullscreen,
            file_menu: false,
            expanded: HashSet::new(),
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let index = self.active;
        self.tabs.get_mut(index)
    }

    pub fn snapshot(&self, order: u32) -> WindowSnapshot {
        let active_tab = self
            .active_tab()
            .map(|tab| tab.id.clone())
            .unwrap_or_default();
        WindowSnapshot {
            label: self.label.clone(),
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            fullscreen: self.fullscreen,
            order,
            active_tab,
            tabs: self.tabs.iter().map(Tab::snapshot).collect(),
            schema_width: Some(self.schema_width as f64),
            editor_width: Some(self.editor_width as f64),
            editor_height: Some(self.editor_height as f64),
        }
    }
}

impl Tab {
    pub fn fresh() -> Self {
        Self {
            id: format!("tab-{}", uuid::Uuid::new_v4()),
            connection_name: None,
            content: Content::with_text(""),
            layout: "all".into(),
            column_widths: BTreeMap::new(),
            file_path: None,
            saved_sql: None,
            newline: "\n".into(),
            bom: false,
            seen_modified: None,
            results: Vec::new(),
            show_rows: logic::VISIBLE_ROWS,
            running: false,
            run_id: String::new(),
            footer: String::new(),
            restored: false,
            disk_notice: false,
            diagnostics: Vec::new(),
            lint_generation: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn from_snapshot(snapshot: TabSnapshot) -> Self {
        let layout = if logic::known_layout(&snapshot.layout) {
            snapshot.layout
        } else {
            "all".into()
        };
        let mut column_widths = BTreeMap::new();
        for (name, width) in snapshot.column_widths {
            if width.is_finite() {
                column_widths.insert(name, width.max(logic::COLUMN_MIN));
            }
        }
        let mut tab = Self {
            id: snapshot.id,
            connection_name: snapshot.connection_name,
            content: Content::with_text(&snapshot.sql),
            layout,
            column_widths,
            file_path: snapshot.file_path,
            saved_sql: snapshot.saved_sql,
            newline: if snapshot.newline.is_empty() {
                "\n".into()
            } else {
                snapshot.newline
            },
            bom: false,
            seen_modified: None,
            results: Vec::new(),
            show_rows: logic::VISIBLE_ROWS,
            running: false,
            run_id: String::new(),
            footer: String::new(),
            restored: !snapshot.sql.is_empty(),
            disk_notice: false,
            diagnostics: Vec::new(),
            lint_generation: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        };
        place_cursor(
            &mut tab.content,
            &snapshot.sql,
            snapshot.cursor,
            snapshot.selection_anchor,
            snapshot.selection_head,
        );
        tab
    }

    pub fn sql(&self) -> String {
        self.content.text()
    }

    pub fn label(&self) -> String {
        logic::tab_label(&self.sql())
    }

    pub fn is_dirty(&self) -> bool {
        is_dirty(
            &self.sql(),
            self.file_path.as_deref(),
            self.saved_sql.as_deref(),
        )
    }

    pub fn needs_file_save(&self) -> bool {
        needs_file_save(
            &self.sql(),
            self.file_path.as_deref(),
            self.saved_sql.as_deref(),
        )
    }

    pub fn cursor_offset(&self) -> usize {
        let cursor = self.content.cursor();
        logic::position_to_offset(&self.sql(), cursor.position.line, cursor.position.column)
    }

    pub fn sql_to_run(&self, statement: bool) -> String {
        let sql = self.sql();
        if statement {
            return statement_at(&sql, self.cursor_offset())
                .map(|part| part.sql)
                .unwrap_or_default();
        }
        if let Some(selected) = self.content.selection()
            && !selected.trim().is_empty()
        {
            return selected;
        }
        sql
    }

    pub fn replace_sql(&mut self, text: &str) {
        self.content = Content::with_text(text);
        self.undo.clear();
        self.redo.clear();
        self.restored = false;
    }

    pub fn remember_undo(&mut self) {
        let current = self.sql();
        if self.undo.last().is_none_or(|saved| saved != &current) {
            self.undo.push(current);
            if self.undo.len() > 40 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    pub fn apply_query_event(&mut self, event: QueryEvent) {
        match event {
            QueryEvent::Begin { index, sql, .. } => {
                let section = self.section_mut(index);
                section.sql = sql;
            }
            QueryEvent::Columns { index, columns } => {
                self.section_mut(index).columns = columns;
            }
            QueryEvent::Rows { index, rows } => {
                self.section_mut(index).rows.extend(rows);
            }
            QueryEvent::StatementDone {
                index,
                rows_affected,
                elapsed_ms,
                truncated,
                ..
            } => {
                let section = self.section_mut(index);
                section.rows_affected = rows_affected;
                section.elapsed_ms = Some(elapsed_ms);
                section.truncated = truncated;
            }
            QueryEvent::StatementError { index, message } => {
                self.section_mut(index).error = Some(message);
            }
            QueryEvent::Finished {
                executed,
                total,
                failed,
                elapsed_ms,
                cancelled,
                ..
            } => {
                let summary = logic::execution_summary(total, executed, failed);
                let elapsed = logic::format_elapsed(Some(elapsed_ms));
                let text = if elapsed.is_empty() {
                    summary
                } else {
                    format!("{summary} · {elapsed}")
                };
                self.footer = if cancelled {
                    format!("Cancelled · {text}")
                } else {
                    text
                };
                self.running = false;
                self.restored = false;
            }
        }
    }

    pub fn snapshot(&self) -> TabSnapshot {
        let sql = self.sql();
        let cursor = self.content.cursor();
        let head = logic::position_to_offset(&sql, cursor.position.line, cursor.position.column);
        let anchor = cursor
            .selection
            .map(|position| logic::position_to_offset(&sql, position.line, position.column) as u32);
        TabSnapshot {
            id: self.id.clone(),
            connection_name: self.connection_name.clone(),
            sql,
            cursor: head as u32,
            selection_anchor: anchor,
            selection_head: cursor.selection.map(|_| head as u32),
            scroll_top: 0.0,
            layout: self.layout.clone(),
            column_widths: self.column_widths.clone(),
            file_path: self.file_path.clone(),
            saved_sql: self.saved_sql.clone(),
            dirty: self.is_dirty(),
            newline: self.newline.clone(),
        }
    }

    fn section_mut(&mut self, index: usize) -> &mut ResultSection {
        while self.results.len() <= index {
            self.results.push(ResultSection::empty());
        }
        &mut self.results[index]
    }
}

impl ResultSection {
    pub fn empty() -> Self {
        Self {
            sql: String::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            error: None,
            rows_affected: None,
            elapsed_ms: None,
            truncated: false,
        }
    }
}

impl ConnForm {
    pub fn blank() -> Self {
        Self {
            editing: None,
            name: String::new(),
            host: "127.0.0.1".into(),
            port: "3306".into(),
            user: String::new(),
            password: String::new(),
            schema: String::new(),
            color: String::new(),
            ssl: false,
            explain_lint: false,
            status: String::new(),
        }
    }

    pub fn from_connection(connection: &Connection) -> Self {
        Self {
            editing: Some(connection.name.clone()),
            name: connection.name.clone(),
            host: connection.host.clone(),
            port: connection.port.to_string(),
            user: connection.user.clone(),
            password: String::new(),
            schema: connection.schema.clone().unwrap_or_default(),
            color: connection.color.clone().unwrap_or_default(),
            ssl: connection.ssl,
            explain_lint: connection.explain_lint,
            status: String::new(),
        }
    }
}

fn place_cursor(
    content: &mut Content,
    sql: &str,
    cursor: u32,
    anchor: Option<u32>,
    head: Option<u32>,
) {
    let caret = head.unwrap_or(cursor) as usize;
    let (line, column) = logic::offset_to_position(sql, caret);
    let selection = anchor.and_then(|anchor| {
        if Some(anchor) == head {
            return None;
        }
        let (line, column) = logic::offset_to_position(sql, anchor as usize);
        Some(Position { line, column })
    });
    content.move_to(Cursor {
        position: Position { line, column },
        selection,
    });
}

pub fn load_session(dir: &std::path::Path) -> SessionDocument {
    let path = dir.join("session.json");
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn primary_dir(primary: &Primary) -> PathBuf {
    let _ = primary;
    sqlg_core::data_dir()
}
