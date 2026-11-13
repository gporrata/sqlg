use std::path::PathBuf;

use iced::window;
use iced::{Point, Size};
use sqlg_core::Diagnostic;
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
    ApplyStartup,
    Edit {
        window: window::Id,
        tab: String,
        action: iced::widget::text_editor::Action,
    },
    Undo {
        window: window::Id,
        tab: String,
    },
    Redo {
        window: window::Id,
        tab: String,
    },
    Run {
        window: window::Id,
        tab: Option<String>,
        statement: bool,
    },
    Cancel {
        window: window::Id,
        tab: String,
    },
    Query {
        window: window::Id,
        tab: String,
        run_id: String,
        event: QueryEvent,
    },
    NewTab(window::Id),
    NewWindow,
    CloseTab(window::Id),
    CloseWindow(window::Id),
    SelectTab {
        window: window::Id,
        index: usize,
    },
    SetConnection {
        window: window::Id,
        tab: String,
        name: String,
    },
    SetLayout {
        window: window::Id,
        layout: String,
    },
    ToggleTheme,
    ToggleFileMenu(window::Id),
    Open(window::Id),
    Save(window::Id),
    SaveAs(window::Id),
    SaveSelection(window::Id),
    FilePicked {
        window: window::Id,
        tab: String,
        purpose: FilePurpose,
        path: Option<PathBuf>,
    },
    Drop {
        window: window::Id,
        path: PathBuf,
    },
    ReloadDisk {
        window: window::Id,
        tab: String,
    },
    DismissDisk {
        window: window::Id,
        tab: String,
    },
    InsertText {
        window: window::Id,
        tab: String,
        text: String,
    },
    ToggleExpand {
        window: window::Id,
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
        window: window::Id,
    },
    LintReady {
        window: window::Id,
        tab: String,
        generation: u64,
        diagnostics: Vec<Diagnostic>,
    },
    ExplainReady {
        window: window::Id,
        tab: String,
        generation: u64,
        result: Result<(), String>,
    },
    ApplyFix {
        window: window::Id,
        tab: String,
        from: usize,
        to: usize,
        text: String,
    },
    Copy(String),
    ShowMore {
        window: window::Id,
        tab: String,
    },
    OpenConnections(window::Id),
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
        window: window::Id,
        kind: DragKind,
    },
    DragMove(Point),
    DragEnd,
    Quit,
    Escape(window::Id),
    WindowOpened {
        window: window::Id,
        position: Option<Point>,
        size: Size,
    },
    WindowClosed(window::Id),
    Moved {
        window: window::Id,
        point: Point,
    },
    Resized {
        window: window::Id,
        size: Size,
    },
    Focused(window::Id),
}
