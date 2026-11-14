use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use sqlg_core::{Connection, Diagnostic, LintCatalog};
use sqlg_mysql::{QueryEvent, SchemaSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Host,
    Port,
    User,
    Password,
    Schema,
    Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePurpose {
    Open,
    SaveAs,
    SaveSelection { text: String },
}

#[derive(Debug, Clone)]
pub enum DragKind {
    Schema,
    EditorWidth,
    EditorHeight,
    Column { tab: String, name: String },
}

#[derive(Debug, Clone)]
pub enum Message {
    Nop,
    Tick,
    Edit {
        window: u64,
        tab: String,
        text: String,
    },
    Selection {
        window: u64,
        tab: String,
        anchor: usize,
        head: usize,
    },
    Undo {
        window: u64,
        tab: String,
    },
    Redo {
        window: u64,
        tab: String,
    },
    Run {
        window: u64,
        tab: Option<String>,
        statement: bool,
    },
    Cancel {
        window: u64,
        tab: String,
    },
    Query {
        window: u64,
        tab: String,
        run_id: String,
        event: QueryEvent,
    },
    NewTab(u64),
    NewWindow,
    CloseTab(u64),
    CloseWindow(u64),
    SelectTab {
        window: u64,
        index: usize,
    },
    SetConnection {
        window: u64,
        tab: String,
        name: String,
    },
    SetLayout {
        window: u64,
        layout: String,
    },
    SetTheme(String),
    ToggleFileMenu(u64),
    Open(u64),
    Save(u64),
    SaveAs(u64),
    SaveSelection(u64),
    FilePicked {
        window: u64,
        tab: String,
        purpose: FilePurpose,
        path: Option<PathBuf>,
    },
    Drop {
        window: u64,
        path: PathBuf,
    },
    ReloadDisk {
        window: u64,
        tab: String,
    },
    DismissDisk {
        window: u64,
        tab: String,
    },
    InsertText {
        window: u64,
        tab: String,
        text: String,
    },
    ToggleExpand {
        window: u64,
        key: String,
    },
    SchemaLoaded {
        name: String,
        result: Result<SchemaSnapshot, String>,
    },
    Pinged {
        name: String,
        ok: bool,
    },
    ReloadSchema {
        window: u64,
    },
    LintReady {
        window: u64,
        tab: String,
        generation: u64,
        diagnostics: Vec<Diagnostic>,
    },
    ExplainReady {
        window: u64,
        tab: String,
        generation: u64,
        result: Result<(), String>,
    },
    ApplyFix {
        window: u64,
        tab: String,
        from: usize,
        to: usize,
        text: String,
    },
    Copied,
    ShowMore {
        window: u64,
        tab: String,
    },
    OpenConnections(u64),
    FormNew,
    FormPick(String),
    FormInput {
        field: Field,
        value: String,
    },
    FormFlag {
        ssl: Option<bool>,
        explain: Option<bool>,
    },
    FormSave,
    FormTest,
    FormDelete,
    FormTested(Result<String, String>),
    DismissModal,
    CloseChoice {
        save: bool,
    },
    InstallCli,
    UninstallCli,
    CliResult(Result<String, String>),
    StartDrag {
        window: u64,
        kind: DragKind,
        x: f32,
        y: f32,
    },
    DragMove {
        x: f32,
        y: f32,
    },
    DragEnd,
    Quit,
    Escape(u64),
    Moved {
        window: u64,
        x: f64,
        y: f64,
    },
    Resized {
        window: u64,
        width: f64,
        height: f64,
    },
    Focused(u64),
}

pub struct QueryJob {
    pub window: u64,
    pub tab: String,
    pub run_id: String,
    pub sql: String,
    pub connection: String,
    pub cancel: Arc<AtomicBool>,
    pub connection_id: Arc<AtomicU64>,
}

pub struct LintJob {
    pub window: u64,
    pub tab: String,
    pub generation: u64,
    pub sql: String,
    pub catalog: LintCatalog,
}

pub struct ExplainJob {
    pub window: u64,
    pub tab: String,
    pub generation: u64,
    pub name: String,
    pub sql: String,
}

pub enum Effect {
    Query(QueryJob),
    Cancel(String),
    Schema(String),
    Ping(String),
    Lint(LintJob),
    Explain(ExplainJob),
    Test {
        connection: Connection,
        password: String,
    },
    Pick {
        window: u64,
        tab: String,
        purpose: FilePurpose,
        save: bool,
    },
    OpenWindow(u64),
    Focus(u64),
    Close(u64),
    Exit,
    Install,
    Uninstall,
}
