use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

use sqlg_core::{
    Connection, Diagnostic, EditorSnapshot, SessionDocument, TabSnapshot, WindowSnapshot,
    statement_at,
};
use sqlg_mysql::{ColumnInfo, QueryEvent, SchemaSnapshot};

use crate::db::Db;
use crate::ipc::{Incoming, Primary};
use crate::logic::{self, is_dirty, needs_file_save};
use crate::message::DragKind;

pub struct App {
    pub windows: Vec<GuiWindow>,
    pub root_id: u64,
    pub next_id: u64,
    pub front: Option<u64>,
    pub theme_name: String,
    pub connections: Vec<Connection>,
    pub connection_names: Vec<String>,
    pub schemas: HashMap<String, SchemaSnapshot>,
    pub schema_errors: HashMap<String, String>,
    pub schema_loading: HashSet<String>,
    pub pings: HashMap<String, bool>,
    pub db: Arc<Db>,
    pub modal: Modal,
    pub form_generation: u64,
    pub drag: Option<Drag>,
    pub status: String,
    pub data_dir: PathBuf,
    pub tick: u64,
    pub startup: Vec<sqlg_core::CliRequest>,
    pub incoming: Receiver<Incoming>,
    pub _lock: std::fs::File,
}

pub struct GuiWindow {
    pub id: u64,
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
    pub schema_query: String,
    pub schema_match: usize,
}

pub struct Tab {
    pub id: String,
    pub connection_name: Option<String>,
    pub buffer: Buffer,
    pub layout: String,
    pub column_widths: BTreeMap<String, f64>,
    pub file_path: Option<String>,
    pub saved_sql: Option<String>,
    pub newline: String,
    pub bom: bool,
    pub seen_modified: Option<SystemTime>,
    pub editors: BTreeMap<String, StoredEditor>,
    pub decks: Vec<ResultDeck>,
    pub active_deck: usize,
    pub run_deck: u64,
    pub next_deck: u64,
    pub running: bool,
    pub run_id: String,
    pub restored: bool,
    pub disk_notice: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub lint_generation: u64,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StoredEditor {
    pub buffer: Buffer,
    pub file_path: Option<String>,
    pub saved_sql: Option<String>,
    pub newline: String,
    pub bom: bool,
    pub seen_modified: Option<SystemTime>,
    pub decks: Vec<ResultDeck>,
    pub active_deck: usize,
    pub run_deck: u64,
    pub next_deck: u64,
    pub running: bool,
    pub run_id: String,
    pub restored: bool,
    pub disk_notice: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ResultDeck {
    pub id: u64,
    pub pinned: bool,
    pub label: String,
    pub results: Vec<ResultSection>,
    pub show_rows: usize,
    pub footer: String,
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub text: String,
    pub anchor: usize,
    pub head: usize,
    pub revision: u64,
}

#[derive(Clone, Debug)]
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
    pub window: u64,
    pub tab_ids: Vec<String>,
    pub close_window: bool,
    pub quit: bool,
}

pub struct Drag {
    pub window: u64,
    pub kind: DragKind,
    pub origin: f32,
    pub origin_size: f32,
    pub horizontal: bool,
}

impl App {
    pub fn boot(primary: Primary) -> Self {
        let data_dir = sqlg_core::data_dir();
        let session = load_session(&data_dir);
        let db = Arc::new(Db::new(data_dir.clone()));
        let mut app = Self {
            windows: Vec::new(),
            root_id: 0,
            next_id: 0,
            front: None,
            theme_name: crate::theme::find_theme(&session.theme).id.into(),
            connections: Vec::new(),
            connection_names: Vec::new(),
            schemas: HashMap::new(),
            schema_errors: HashMap::new(),
            schema_loading: HashSet::new(),
            pings: HashMap::new(),
            db,
            modal: Modal::None,
            form_generation: 0,
            drag: None,
            status: String::new(),
            data_dir,
            tick: 0,
            startup: primary.startup,
            incoming: primary.incoming,
            _lock: primary._lock,
        };
        app.reload_connections();
        let snapshots = session.windows;
        let front = session.front_window;
        if snapshots.is_empty() {
            let id = app.alloc_id();
            app.windows.push(GuiWindow::fresh(id));
            app.root_id = id;
            app.front = Some(id);
        } else {
            for (index, snapshot) in snapshots.into_iter().enumerate() {
                let id = app.alloc_id();
                if index == 0 {
                    app.root_id = id;
                }
                app.windows.push(GuiWindow::from_snapshot(id, snapshot));
            }
            app.front = front
                .as_ref()
                .and_then(|label| app.windows.iter().find(|gui| &gui.label == label))
                .map(|gui| gui.id)
                .or_else(|| app.windows.first().map(|gui| gui.id));
        }
        app
    }

    pub fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn window(&self, id: u64) -> Option<&GuiWindow> {
        self.windows.iter().find(|window| window.id == id)
    }

    pub fn window_mut(&mut self, id: u64) -> Option<&mut GuiWindow> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    pub fn tab_mut(&mut self, id: u64, tab: &str) -> Option<&mut Tab> {
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

    pub fn dirty_tabs(&self, window: Option<u64>) -> Vec<(u64, String)> {
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

    pub fn show_connections(&mut self, form: ConnForm) {
        self.form_generation = self.form_generation.wrapping_add(1);
        self.modal = Modal::Connections(form);
    }

    pub fn title(&self, id: u64) -> String {
        self.window(id)
            .and_then(|gui| gui.active_tab())
            .map(|tab| format!("sqlg — {}", tab.label()))
            .unwrap_or_else(|| "sqlg".into())
    }
}

impl GuiWindow {
    pub fn fresh(id: u64) -> Self {
        Self::with_tabs(id, vec![Tab::fresh()])
    }

    pub fn with_tabs(id: u64, tabs: Vec<Tab>) -> Self {
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
            schema_query: String::new(),
            schema_match: 0,
        }
    }

    pub fn from_snapshot(id: u64, snapshot: WindowSnapshot) -> Self {
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
            schema_query: String::new(),
            schema_match: 0,
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
            buffer: Buffer::new(""),
            layout: "all".into(),
            column_widths: BTreeMap::new(),
            file_path: None,
            saved_sql: None,
            newline: "\n".into(),
            bom: false,
            seen_modified: None,
            editors: BTreeMap::new(),
            decks: Vec::new(),
            active_deck: 0,
            run_deck: 0,
            next_deck: 0,
            running: false,
            run_id: String::new(),
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
        let mut buffer = Buffer::new(&snapshot.sql);
        let head = snapshot.selection_head.unwrap_or(snapshot.cursor) as usize;
        let anchor = snapshot.selection_anchor.unwrap_or(head as u32) as usize;
        buffer.head = floor_boundary(&buffer.text, head);
        buffer.anchor = if snapshot.selection_anchor.is_some()
            && snapshot.selection_anchor != snapshot.selection_head
        {
            floor_boundary(&buffer.text, anchor)
        } else {
            buffer.head
        };
        let mut editors = BTreeMap::new();
        for editor in snapshot.editors {
            editors.insert(
                editor.connection_name.clone(),
                StoredEditor::from_snapshot(editor),
            );
        }
        Self {
            id: snapshot.id,
            connection_name: snapshot.connection_name,
            buffer,
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
            editors,
            decks: Vec::new(),
            active_deck: 0,
            run_deck: 0,
            next_deck: 0,
            running: false,
            run_id: String::new(),
            restored: !snapshot.sql.is_empty(),
            disk_notice: false,
            diagnostics: Vec::new(),
            lint_generation: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn sql(&self) -> String {
        self.buffer.text.clone()
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
        ) || self.editors.values().any(StoredEditor::needs_file_save)
    }

    pub fn switch_connection(&mut self, name: Option<String>) {
        let next = name.clone().unwrap_or_default();
        let current = self.connection_name.clone().unwrap_or_default();
        if next == current {
            return;
        }
        let stored = self.capture_editor();
        self.editors.insert(current, stored);
        self.connection_name = name;
        if let Some(stored) = self.editors.remove(&next) {
            self.restore_editor(stored);
        } else {
            self.clear_editor();
        }
    }

    pub fn current_deck(&self) -> Option<&ResultDeck> {
        if self.decks.is_empty() {
            return None;
        }
        self.decks.get(self.active_deck.min(self.decks.len() - 1))
    }

    pub fn prepare_run(&mut self) {
        let index = if self.decks.is_empty() {
            self.push_deck()
        } else {
            let active = self.active_deck.min(self.decks.len() - 1);
            if self.decks[active].pinned {
                self.push_deck()
            } else {
                active
            }
        };
        self.active_deck = index;
        let deck = &mut self.decks[index];
        deck.results.clear();
        deck.show_rows = logic::VISIBLE_ROWS;
        deck.footer = "Running…".into();
        self.run_deck = deck.id;
        self.running = true;
        self.restored = false;
    }

    pub fn set_idle_footer(&mut self, text: &str) -> bool {
        if self.decks.is_empty() {
            return false;
        }
        let index = self.active_deck.min(self.decks.len() - 1);
        if self.decks[index].pinned {
            return false;
        }
        self.decks[index].footer = text.to_string();
        true
    }

    pub fn select_deck(&mut self, index: usize) {
        if index < self.decks.len() {
            self.active_deck = index;
        }
    }

    pub fn pin_deck(&mut self, index: usize) {
        if let Some(deck) = self.decks.get_mut(index) {
            deck.pinned = true;
        }
    }

    pub fn unpin_deck(&mut self, index: usize) {
        if index >= self.decks.len() {
            return;
        }
        self.decks.remove(index);
        if self.decks.is_empty() {
            self.active_deck = 0;
            return;
        }
        if self.active_deck > index {
            self.active_deck -= 1;
        } else if self.active_deck >= self.decks.len() {
            self.active_deck = self.decks.len() - 1;
        }
    }

    fn push_deck(&mut self) -> usize {
        self.next_deck = self.next_deck.saturating_add(1);
        let id = self.next_deck;
        self.decks.push(ResultDeck {
            id,
            pinned: false,
            label: format!("Result {id}"),
            results: Vec::new(),
            show_rows: logic::VISIBLE_ROWS,
            footer: String::new(),
        });
        self.decks.len() - 1
    }

    fn capture_editor(&self) -> StoredEditor {
        StoredEditor {
            buffer: self.buffer.clone(),
            file_path: self.file_path.clone(),
            saved_sql: self.saved_sql.clone(),
            newline: self.newline.clone(),
            bom: self.bom,
            seen_modified: self.seen_modified,
            decks: self.decks.clone(),
            active_deck: self.active_deck,
            run_deck: self.run_deck,
            next_deck: self.next_deck,
            running: self.running,
            run_id: self.run_id.clone(),
            restored: self.restored,
            disk_notice: self.disk_notice,
            diagnostics: self.diagnostics.clone(),
            undo: self.undo.clone(),
            redo: self.redo.clone(),
        }
    }

    fn restore_editor(&mut self, stored: StoredEditor) {
        self.buffer = stored.buffer;
        self.file_path = stored.file_path;
        self.saved_sql = stored.saved_sql;
        self.newline = stored.newline;
        self.bom = stored.bom;
        self.seen_modified = stored.seen_modified;
        self.decks = stored.decks;
        self.active_deck = stored.active_deck;
        self.run_deck = stored.run_deck;
        self.next_deck = stored.next_deck;
        self.running = stored.running;
        self.run_id = stored.run_id;
        self.restored = stored.restored;
        self.disk_notice = stored.disk_notice;
        self.diagnostics = stored.diagnostics;
        self.undo = stored.undo;
        self.redo = stored.redo;
    }

    fn clear_editor(&mut self) {
        let revision = self.buffer.revision.wrapping_add(1);
        self.buffer = Buffer::new("");
        self.buffer.revision = revision;
        self.file_path = None;
        self.saved_sql = None;
        self.newline = "\n".into();
        self.bom = false;
        self.seen_modified = None;
        self.decks.clear();
        self.active_deck = 0;
        self.run_deck = 0;
        self.running = false;
        self.run_id.clear();
        self.restored = false;
        self.disk_notice = false;
        self.diagnostics.clear();
        self.undo.clear();
        self.redo.clear();
    }

    pub fn cursor_offset(&self) -> usize {
        self.buffer.head.min(self.buffer.text.len())
    }

    pub fn selected_text(&self) -> Option<String> {
        self.buffer.selected_text()
    }

    pub fn sql_to_run(&self, statement: bool) -> String {
        let sql = self.sql();
        if statement {
            return statement_at(&sql, self.cursor_offset())
                .map(|part| part.sql)
                .unwrap_or_default();
        }
        if let Some(selected) = self.selected_text()
            && !selected.trim().is_empty()
        {
            return selected;
        }
        sql
    }

    pub fn replace_sql(&mut self, text: &str) {
        self.buffer.replace_text(text);
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
        apply_deck_event(
            &mut self.decks,
            self.run_deck,
            &mut self.running,
            &mut self.restored,
            event,
        );
    }

    pub fn snapshot(&self) -> TabSnapshot {
        let sql = self.sql();
        let head = self.cursor_offset();
        let anchor = if self.buffer.anchor == self.buffer.head {
            None
        } else {
            Some(self.buffer.anchor as u32)
        };
        TabSnapshot {
            id: self.id.clone(),
            connection_name: self.connection_name.clone(),
            sql,
            cursor: head as u32,
            selection_anchor: anchor,
            selection_head: anchor.map(|_| head as u32),
            scroll_top: 0.0,
            layout: self.layout.clone(),
            column_widths: self.column_widths.clone(),
            file_path: self.file_path.clone(),
            saved_sql: self.saved_sql.clone(),
            dirty: self.is_dirty(),
            newline: self.newline.clone(),
            editors: self
                .editors
                .iter()
                .filter(|(_, editor)| editor.keeps_text())
                .map(|(name, editor)| editor.snapshot(name))
                .collect(),
        }
    }
}

impl StoredEditor {
    fn from_snapshot(snapshot: EditorSnapshot) -> Self {
        let mut buffer = Buffer::new(&snapshot.sql);
        let head = snapshot.selection_head.unwrap_or(snapshot.cursor) as usize;
        let anchor = snapshot.selection_anchor.unwrap_or(head as u32) as usize;
        buffer.head = floor_boundary(&buffer.text, head);
        buffer.anchor = if snapshot.selection_anchor.is_some()
            && snapshot.selection_anchor != snapshot.selection_head
        {
            floor_boundary(&buffer.text, anchor)
        } else {
            buffer.head
        };
        Self {
            buffer,
            file_path: snapshot.file_path,
            saved_sql: snapshot.saved_sql,
            newline: if snapshot.newline.is_empty() {
                "\n".into()
            } else {
                snapshot.newline
            },
            bom: false,
            seen_modified: None,
            decks: Vec::new(),
            active_deck: 0,
            run_deck: 0,
            next_deck: 0,
            running: false,
            run_id: String::new(),
            restored: !snapshot.sql.is_empty(),
            disk_notice: false,
            diagnostics: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn apply_query_event(&mut self, event: QueryEvent) {
        apply_deck_event(
            &mut self.decks,
            self.run_deck,
            &mut self.running,
            &mut self.restored,
            event,
        );
    }

    fn needs_file_save(&self) -> bool {
        needs_file_save(
            &self.buffer.text,
            self.file_path.as_deref(),
            self.saved_sql.as_deref(),
        )
    }

    fn keeps_text(&self) -> bool {
        !self.buffer.text.is_empty() || self.file_path.is_some() || self.saved_sql.is_some()
    }

    fn snapshot(&self, connection_name: &str) -> EditorSnapshot {
        let head = self.buffer.head.min(self.buffer.text.len());
        let anchor = if self.buffer.anchor == self.buffer.head {
            None
        } else {
            Some(self.buffer.anchor as u32)
        };
        EditorSnapshot {
            connection_name: connection_name.to_string(),
            sql: self.buffer.text.clone(),
            cursor: head as u32,
            selection_anchor: anchor,
            selection_head: anchor.map(|_| head as u32),
            file_path: self.file_path.clone(),
            saved_sql: self.saved_sql.clone(),
            newline: self.newline.clone(),
        }
    }
}

fn apply_deck_event(
    decks: &mut [ResultDeck],
    run_deck: u64,
    running: &mut bool,
    restored: &mut bool,
    event: QueryEvent,
) {
    match event {
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
            if let Some(deck) = decks.iter_mut().find(|deck| deck.id == run_deck) {
                deck.footer = if cancelled {
                    format!("Cancelled · {text}")
                } else {
                    text
                };
            }
            *running = false;
            *restored = false;
        }
        QueryEvent::Begin { index, sql, .. } => {
            if let Some(section) = section_mut(decks, run_deck, index) {
                section.sql = sql;
            }
        }
        QueryEvent::Columns { index, columns } => {
            if let Some(section) = section_mut(decks, run_deck, index) {
                section.columns = columns;
            }
        }
        QueryEvent::Rows { index, rows } => {
            if let Some(section) = section_mut(decks, run_deck, index) {
                section.rows.extend(rows);
            }
        }
        QueryEvent::StatementDone {
            index,
            rows_affected,
            elapsed_ms,
            truncated,
            ..
        } => {
            if let Some(section) = section_mut(decks, run_deck, index) {
                section.rows_affected = rows_affected;
                section.elapsed_ms = Some(elapsed_ms);
                section.truncated = truncated;
            }
        }
        QueryEvent::StatementError { index, message } => {
            if let Some(section) = section_mut(decks, run_deck, index) {
                section.error = Some(message);
            }
        }
    }
}

fn section_mut(
    decks: &mut [ResultDeck],
    run_deck: u64,
    index: usize,
) -> Option<&mut ResultSection> {
    let deck = decks.iter_mut().find(|deck| deck.id == run_deck)?;
    while deck.results.len() <= index {
        deck.results.push(ResultSection::empty());
    }
    Some(&mut deck.results[index])
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            anchor: 0,
            head: 0,
            revision: 0,
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = ordered(self.anchor, self.head);
        if start == end {
            return None;
        }
        self.text.get(start..end).map(str::to_string)
    }

    pub fn set_typed(&mut self, text: String) {
        self.text = text;
        self.anchor = floor_boundary(&self.text, self.anchor);
        self.head = floor_boundary(&self.text, self.head);
    }

    pub fn replace_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.anchor = self.text.len();
        self.head = self.text.len();
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn set_utf16_span(&mut self, anchor: usize, head: usize) {
        self.anchor = logic::utf16_to_byte(&self.text, anchor);
        self.head = logic::utf16_to_byte(&self.text, head);
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

pub fn load_session(dir: &std::path::Path) -> SessionDocument {
    let path = dir.join("session.json");
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn ordered(anchor: usize, head: usize) -> (usize, usize) {
    if anchor <= head {
        (anchor, head)
    } else {
        (head, anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_connections_keeps_each_editor() {
        let mut tab = Tab::fresh();
        tab.buffer.set_typed("select a".into());
        tab.switch_connection(Some("dev".into()));
        assert_eq!(tab.sql(), "");
        tab.buffer.set_typed("select b".into());
        tab.switch_connection(Some("prod".into()));
        tab.buffer.set_typed("select c".into());
        tab.switch_connection(Some("dev".into()));
        assert_eq!(tab.sql(), "select b");
        tab.switch_connection(None);
        assert_eq!(tab.sql(), "select a");
        let snapshot = tab.snapshot();
        assert_eq!(snapshot.sql, "select a");
        assert_eq!(snapshot.editors.len(), 2);
        tab.switch_connection(Some("dev".into()));
        assert_eq!(tab.sql(), "select b");
    }

    #[test]
    fn pinned_result_opens_another_and_unpin_removes_it() {
        let mut tab = Tab::fresh();
        tab.prepare_run();
        assert_eq!(tab.decks.len(), 1);
        assert_eq!(tab.decks[0].label, "Result 1");
        tab.pin_deck(0);
        tab.prepare_run();
        assert_eq!(tab.decks.len(), 2);
        assert!(tab.decks[0].pinned);
        assert!(!tab.decks[1].pinned);
        assert_eq!(tab.active_deck, 1);
        tab.prepare_run();
        assert_eq!(tab.decks.len(), 2);
        tab.pin_deck(1);
        tab.prepare_run();
        assert_eq!(tab.decks.len(), 3);
        assert_eq!(tab.active_deck, 2);
        tab.select_deck(0);
        tab.unpin_deck(0);
        assert_eq!(tab.decks.len(), 2);
        assert!(tab.decks.iter().all(|deck| deck.id != 1));
    }
}

fn floor_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    if text.is_char_boundary(offset) {
        offset
    } else {
        text.char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= offset)
            .last()
            .unwrap_or(0)
    }
}
