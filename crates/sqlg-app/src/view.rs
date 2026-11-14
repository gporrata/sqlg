use dioxus::prelude::*;
use sqlg_core::{SqlTone, highlight_sql};
use sqlg_mysql::SchemaSnapshot;

use crate::logic::{self, ColumnMark, Panes};
use crate::message::{DragKind, Field, Message};
use crate::model::{Modal, ResultSection};
use crate::runtime;
use crate::theme;

const EDITOR_ID: &str = "sql-editor";

const LAYOUTS: &[(&str, &str)] = &[
    ("all", "All"),
    ("editor-results", "Split"),
    ("below", "Below"),
    ("below-only", "Stack"),
    ("editor", "Editor"),
    ("results", "Results"),
];

pub const HEAD: &str = r#"<style>
:root { color-scheme: light; }
.shell {
  --tone-num: #0b6b66;
  --tone-text: #9a4510;
  --tone-time: #b42318;
  --tone-enum: #2d6b1f;
  --tone-other: #5e5e66;
}
.shell.dark {
  color-scheme: dark;
  --tone-num: #9cf0e4;
  --tone-text: #ffc48a;
  --tone-time: #ffb1a8;
  --tone-enum: #b7e3a8;
  --tone-other: #c8c8ce;
}
html, body { height: 100%; margin: 0; }
body { background: var(--bg); color: var(--ink); font: 13px/1.4 -apple-system, BlinkMacSystemFont, sans-serif; }
button, select, input { font: inherit; color: inherit; }
button { background: var(--panel); border: 1px solid var(--line); border-radius: 4px; padding: 3px 8px; }
button:hover { border-color: var(--accent); }
button.active, button.primary { background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }
.shell { height: 100vh; display: flex; flex-direction: column; background: var(--bg); color: var(--ink); }
.toolbar, .footer, .tabs { display: flex; align-items: center; gap: 6px; padding: 6px 8px; background: var(--panel); }
.toolbar, .tabs { border-bottom: 1px solid var(--line); }
.footer { border-top: 1px solid var(--line); }
.menu-wrap { position: relative; }
.menu { position: absolute; z-index: 5; top: 28px; left: 0; min-width: 220px; display: flex; flex-direction: column; gap: 2px; padding: 6px; background: var(--panel); border: 1px solid var(--line); border-radius: 6px; box-shadow: 0 8px 24px rgba(0,0,0,.18); }
.menu button { text-align: left; border: 0; background: transparent; }
.menu button:hover { background: var(--chip); }
.tabs { overflow: auto; }
.tab { display: flex; align-items: center; gap: 6px; max-width: 220px; border-radius: 4px; padding: 3px 6px; background: transparent; border: 1px solid transparent; }
.tab.active { background: var(--chip); border-color: var(--line); }
.tab span.label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tab .close { border: 0; background: transparent; padding: 0 2px; display: none; }
.tab:hover .close, .tab:not(.dirty) .close { display: inline; }
.tab.dirty .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); display: inline-block; }
.tab:hover .dot, .tab:not(.dirty) .dot { display: none; }
.spacer { flex: 1; }
.badge { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }
.missing { color: var(--danger); }
.body { flex: 1; display: flex; min-height: 0; }
.body.stack { flex-direction: column; }
.schema, .editor-pane, .results { min-width: 0; min-height: 0; }
.schema { overflow: auto; border-right: 1px solid var(--line); background: var(--panel); display: flex; flex-direction: column; }
.schema .actions { padding: 6px; }
.schema .actions button { display: block; width: 100%; text-align: left; border: 0; background: transparent; padding: 2px 4px; }
.schema .actions button:hover { background: var(--chip); }
.tree { flex: 1; min-height: 0; overflow: auto; position: relative; }
.tn { position: relative; display: flex; align-items: center; gap: 4px; width: calc(100% - 8px); box-sizing: border-box; height: 24px; margin: 0 4px; border-radius: 5px; white-space: nowrap; }
.tn:hover { background: var(--chip); }
.tn .nm { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.ty { flex: none; margin-left: auto; width: var(--type-ch, auto); white-space: nowrap; text-align: left; color: var(--tone-other); font: 10.5px Menlo, monospace; padding-right: 6px; }
.tone-num { color: var(--tone-num); }
.tone-text { color: var(--tone-text); }
.tone-time { color: var(--tone-time); }
.tone-enum { color: var(--tone-enum); }
.tone-other { color: var(--tone-other); }
.cw { width: 14px; height: 14px; flex: none; display: flex; align-items: center; justify-content: center; color: var(--muted); }
.chev { transition: transform 120ms ease; }
.chev.open { transform: rotate(90deg); }
.ic { flex: none; color: var(--muted); }
.ic.gold { color: #b8860b; }
.ic.purple { color: #7a4ea3; }
.gl { width: 14px; flex: none; text-align: center; font: 700 11px Menlo, monospace; }
.ig { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--line); pointer-events: none; }
.sash { flex: 0 0 6px; background: var(--line); cursor: col-resize; }
.sash.row { cursor: row-resize; }
.editor-pane, .results { display: flex; flex-direction: column; background: var(--editor); }
.sql-editor { position: relative; flex: 1; min-height: 0; }
.sql-highlight, .sql-editor textarea { box-sizing: border-box; margin: 0; padding: 8px; border: 0; width: 100%; height: 100%; font: 13px/1.45 ui-monospace, SFMono-Regular, Menlo, monospace; white-space: pre-wrap; overflow-wrap: normal; tab-size: 4; }
.sql-highlight { position: absolute; inset: 0; overflow: auto; pointer-events: none; color: var(--ink); background: transparent; }
.sql-highlight::-webkit-scrollbar { display: none; }
.sql-editor textarea { position: relative; display: block; resize: none; outline: none; overflow: auto; background: transparent; color: transparent; caret-color: var(--ink); }
.sql-editor textarea::selection { background: color-mix(in srgb, var(--accent) 35%, transparent); color: transparent; }
.sql-kw { color: var(--syn-keyword); }
.sql-str { color: var(--syn-string); }
.sql-num { color: var(--syn-number); }
.sql-com { color: var(--syn-comment); }
.sql-op { color: var(--syn-operator); }
.diagnostics, .banner { padding: 4px 8px; background: var(--panel); border-top: 1px solid var(--line); }
.diagnostic { display: flex; gap: 8px; align-items: baseline; padding: 2px 0; }
.diagnostic.warn { color: var(--warn); }
.diagnostic.error, .error { color: var(--danger); }
.results { overflow: auto; flex: 1; }
.section { margin: 8px; }
.grid { border-collapse: collapse; font: 12px/1.3 ui-monospace, SFMono-Regular, Menlo, monospace; }
.grid th, .grid td { border: 1px solid var(--line); padding: 3px 6px; text-align: left; vertical-align: top; position: relative; }
.grid th { background: var(--chip); }
.grid td.num { text-align: right; }
.grid td.null { color: var(--muted); font-style: italic; }
.col-sash { position: absolute; top: 0; right: 0; width: 6px; height: 100%; cursor: col-resize; }
.placeholder, .muted { color: var(--muted); padding: 8px; }
.footer .summary { flex: 1; text-align: center; color: var(--muted); }
.footer select.theme { background: var(--panel); color: var(--ink); border: 1px solid var(--line); border-radius: 4px; padding: 2px 4px; max-width: 180px; }
.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.35); display: flex; align-items: center; justify-content: center; }
.dialog { width: min(760px, calc(100vw - 48px)); max-height: calc(100vh - 48px); overflow: auto; background: var(--panel); color: var(--ink); border-radius: 8px; padding: 16px; box-shadow: 0 12px 40px rgba(0,0,0,.28); }
.dialog.narrow { width: min(420px, calc(100vw - 48px)); }
.form { display: grid; grid-template-columns: 180px 1fr; gap: 12px; }
.names { display: flex; flex-direction: column; gap: 4px; }
.fields { display: flex; flex-direction: column; gap: 8px; }
.field { display: flex; flex-direction: column; gap: 2px; }
.field input { border: 1px solid var(--line); border-radius: 4px; padding: 4px 6px; background: var(--editor); }
.checks { display: flex; gap: 12px; }
.actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 12px; }
</style>"#;

enum EditorAction {
    Run,
    RunStatement,
    Save,
    SaveAs,
    SaveSelection,
}

pub fn app_root() -> Element {
    let window_id = runtime::root_id();
    rsx! { Shell { window_id } }
}

pub fn child_root() -> Element {
    let window_id = runtime::take_child_id();
    rsx! { Shell { window_id } }
}

#[component]
fn Shell(window_id: u64) -> Element {
    let mut seen = use_signal(|| runtime::epoch());
    use_hook(move || runtime::attach_window(window_id));
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(32)).await;
            let current = runtime::epoch();
            if seen() != current {
                seen.set(current);
            }
        }
    });
    let _redraw = seen();
    let Some(view) = snapshot(window_id) else {
        return rsx! { div {} };
    };
    runtime::sync_chrome(window_id);
    let chosen = theme::find_theme(&view.theme);
    let shell_class = if chosen.dark { "shell dark" } else { "shell" };
    let shell_style = theme::css_vars(chosen);
    rsx! {
        div {
            class: "{shell_class}",
            style: "{shell_style}",
            tabindex: "0",
            onkeydown: move |event| handle_key(window_id, event),
            onmousemove: move |event| drag_move(event),
            onmouseup: move |_| runtime::apply(Message::DragEnd),
            onclick: move |_| close_menu_if_open(window_id),
            Toolbar { window_id, view: view.clone() }
            div {
                class: if view.panes.stack { "body stack" } else { "body" },
                if view.panes.schema {
                    SchemaPane { window_id, view: view.clone() }
                    div {
                        class: "sash",
                        onmousedown: move |event| start_drag(window_id, event, DragKind::Schema),
                    }
                }
                if view.panes.editor {
                    EditorPane { window_id, view: view.clone() }
                }
                if view.panes.editor && view.panes.results {
                    div {
                        class: if view.panes.stack { "sash row" } else { "sash" },
                        onmousedown: move |event| {
                            let kind = if view.panes.stack {
                                DragKind::EditorHeight
                            } else {
                                DragKind::EditorWidth
                            };
                            start_drag(window_id, event, kind);
                        },
                    }
                }
                if view.panes.results {
                    ResultsPane { window_id, view: view.clone() }
                }
            }
            Footer { view: view.clone() }
            ModalLayer { view: view.clone() }
        }
    }
}

#[component]
fn Toolbar(window_id: u64, view: ShellView) -> Element {
    let names = view.connection_names.clone();
    let selected = view.selected_connection.clone();
    let tab_id = view.tab_id.clone();
    rsx! {
        div { class: "toolbar",
            div { class: "menu-wrap",
                button {
                    onclick: move |event| {
                        event.stop_propagation();
                        runtime::apply(Message::ToggleFileMenu(window_id));
                    },
                    "File"
                }
                if view.file_menu {
                    FileMenu { window_id, tab_id: tab_id.clone() }
                }
            }
            div { class: "tabs",
                for tab in view.tabs.clone() {
                    TabChip { window_id, tab }
                }
            }
            select {
                aria_label: "Connection",
                onchange: {
                    let tab_id = tab_id.clone();
                    move |event| {
                        let Some(tab_id) = tab_id.clone() else {
                            return;
                        };
                        runtime::apply(Message::SetConnection {
                            window: window_id,
                            tab: tab_id,
                            name: event.value(),
                        });
                    }
                },
                option { value: "", selected: selected.is_empty(), "Connection" }
                for name in names {
                    option { value: "{name}", selected: name == selected, "{name}" }
                }
            }
            if let Some(color) = view.badge_color.clone() {
                span { class: "badge", style: "background: {color}" }
            }
            if view.connection_missing {
                span { class: "missing", "missing" }
            }
            div { class: "spacer" }
            if view.running {
                button {
                    class: "primary",
                    onclick: {
                        let tab_id = tab_id.clone();
                        move |_| {
                            let Some(tab_id) = tab_id.clone() else {
                                return;
                            };
                            runtime::apply(Message::Cancel { window: window_id, tab: tab_id });
                        }
                    },
                    "Cancel"
                }
            } else {
                button {
                    class: "primary",
                    onclick: {
                        let tab_id = tab_id.clone();
                        move |_| queue_editor(window_id, tab_id.clone(), EditorAction::Run)
                    },
                    "Run"
                }
            }
            div { class: "chiclets",
                for (name, label) in LAYOUTS.iter().copied() {
                    button {
                        class: if view.layout == name { "active" } else { "" },
                        onclick: move |_| runtime::apply(Message::SetLayout {
                            window: window_id,
                            layout: name.to_string(),
                        }),
                        "{label}"
                    }
                }
            }
        }
    }
}

#[component]
fn TabChip(window_id: u64, tab: TabChipView) -> Element {
    let index = tab.index;
    let title = tab.tooltip.clone();
    rsx! {
        div {
            class: if tab.active {
                if tab.dirty { "tab active dirty" } else { "tab active" }
            } else if tab.dirty {
                "tab dirty"
            } else {
                "tab"
            },
            title: "{title}",
            onclick: move |_| runtime::apply(Message::SelectTab { window: window_id, index }),
            span { class: "label", "{tab.label}" }
            span { class: "dot" }
            button {
                class: "close",
                aria_label: "Close tab",
                onclick: move |event| {
                    event.stop_propagation();
                    runtime::apply(Message::SelectTab { window: window_id, index });
                    runtime::apply(Message::CloseTab(window_id));
                },
                "×"
            }
        }
    }
}

#[component]
fn FileMenu(window_id: u64, tab_id: Option<String>) -> Element {
    rsx! {
        div {
            class: "menu",
            onclick: move |event| event.stop_propagation(),
            button { onclick: move |_| runtime::apply(Message::NewTab(window_id)), "New Tab" }
            button { onclick: move |_| runtime::apply(Message::NewWindow), "New Window" }
            button { onclick: move |_| runtime::apply(Message::Open(window_id)), "Open…" }
            button {
                onclick: {
                    let tab_id = tab_id.clone();
                    move |_| queue_editor(window_id, tab_id.clone(), EditorAction::Save)
                },
                "Save"
            }
            button {
                onclick: {
                    let tab_id = tab_id.clone();
                    move |_| queue_editor(window_id, tab_id.clone(), EditorAction::SaveAs)
                },
                "Save As…"
            }
            button {
                onclick: {
                    let tab_id = tab_id.clone();
                    move |_| queue_editor(window_id, tab_id.clone(), EditorAction::SaveSelection)
                },
                "Save Selection As…"
            }
            button { onclick: move |_| runtime::apply(Message::CloseTab(window_id)), "Close Tab" }
            button { onclick: move |_| runtime::apply(Message::CloseWindow(window_id)), "Close Window" }
            button { onclick: move |_| runtime::apply(Message::OpenConnections(window_id)), "Connections…" }
            button { onclick: move |_| runtime::apply(Message::InstallCli), "Install sqlg Command" }
            button { onclick: move |_| runtime::apply(Message::UninstallCli), "Uninstall sqlg Command" }
            button { onclick: move |_| runtime::apply(Message::Quit), "Quit" }
        }
    }
}

#[component]
fn SchemaPane(window_id: u64, view: ShellView) -> Element {
    let width = view.schema_width;
    let type_chars = view
        .schema
        .iter()
        .filter_map(|node| node.type_label.as_deref())
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0);
    let tree_style = if type_chars == 0 {
        String::new()
    } else {
        format!("--type-ch: {type_chars}ch")
    };
    rsx! {
        aside { class: "schema", style: "width: {width}px",
            div { class: "actions",
                button {
                    onclick: move |_| runtime::apply(Message::ReloadSchema { window: window_id }),
                    "Reload"
                }
            }
            if let Some(message) = view.schema_message.clone() {
                p { class: "muted", "{message}" }
            }
            div { class: "tree", style: "{tree_style}",
                for node in view.schema.clone() {
                    SchemaNode { window_id, tab_id: view.tab_id.clone(), node }
                }
            }
        }
    }
}

#[component]
fn SchemaNode(window_id: u64, tab_id: Option<String>, node: SchemaNodeView) -> Element {
    let key = node.key.clone();
    let ident = node.ident.clone();
    let depth = node.depth;
    let pad = 8 + u32::from(depth) * 14;
    let guide = 10 + (i32::from(depth) - 1) * 14;
    let expandable = node.expandable;
    let open = node.open;
    let mark = node.mark;
    let tone = node.tone.clone();
    let type_label = node.type_label.clone();
    rsx! {
        div {
            class: "tn",
            style: "padding-left: {pad}px",
            onclick: {
                let key = key.clone();
                move |_| {
                    if !expandable {
                        return;
                    }
                    runtime::apply(Message::ToggleExpand {
                        window: window_id,
                        key: key.clone(),
                    })
                }
            },
            ondoubleclick: {
                let tab_id = tab_id.clone();
                let ident = ident.clone();
                move |_| {
                    let Some(tab_id) = tab_id.clone() else {
                        return;
                    };
                    let Some(ident) = ident.clone() else {
                        return;
                    };
                    runtime::apply(Message::InsertText {
                        window: window_id,
                        tab: tab_id,
                        text: ident,
                    });
                }
            },
            if depth > 0 {
                i { class: "ig", style: "left: {guide}px" }
            }
            span {
                class: "cw",
                aria_label: if expandable {
                    if open { "Collapse" } else { "Expand" }
                } else {
                    ""
                },
                if expandable {
                    {chevron_mark(open)}
                }
            }
            {kind_mark(mark)}
            span { class: "nm", "{node.label}" }
            if let Some(type_label) = type_label {
                span { class: "ty tone-{tone}", "{type_label}" }
            }
        }
    }
}

fn chevron_mark(open: bool) -> Element {
    rsx! {
        svg {
            class: if open { "chev open" } else { "chev" },
            width: "10",
            height: "10",
            view_box: "0 0 10 10",
            path {
                d: "M3 2l4 3-4 3",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

fn kind_mark(mark: NodeMark) -> Element {
    match mark {
        NodeMark::Group => rsx! {
            svg {
                class: "ic",
                width: "14",
                height: "14",
                view_box: "0 0 16 16",
                ellipse { cx: "8", cy: "4", rx: "5", ry: "2", fill: "none", stroke: "currentColor" }
                path { d: "M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4", fill: "none", stroke: "currentColor" }
            }
        },
        NodeMark::Table => rsx! {
            svg {
                class: "ic",
                width: "14",
                height: "14",
                view_box: "0 0 16 16",
                rect { x: "2", y: "3", width: "12", height: "10", rx: "1", fill: "none", stroke: "currentColor" }
                path { d: "M2 6.5h12M6 6.5V13", stroke: "currentColor" }
            }
        },
        NodeMark::Column(ColumnMark::PrimaryKey) => rsx! {
            svg {
                class: "ic gold",
                width: "14",
                height: "14",
                view_box: "0 0 16 16",
                circle { cx: "5", cy: "8", r: "2.2", fill: "none", stroke: "currentColor" }
                path { d: "M7 8h7M12 8v2M10 8v1.5", stroke: "currentColor", stroke_linecap: "round" }
            }
        },
        NodeMark::Column(ColumnMark::ForeignKey) => rsx! {
            svg {
                class: "ic purple",
                width: "14",
                height: "14",
                view_box: "0 0 16 16",
                path {
                    d: "M6.5 9.5l-1 1a2.2 2.2 0 0 1-3.1-3.1l2-2a2.2 2.2 0 0 1 3.1 0M9.5 6.5l1-1a2.2 2.2 0 0 1 3.1 3.1l-2 2a2.2 2.2 0 0 1-3.1 0",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_linecap: "round",
                }
            }
        },
        NodeMark::Column(ColumnMark::Numeric) => rsx! { span { class: "gl tone-num", "#" } },
        NodeMark::Column(ColumnMark::Text) => rsx! { span { class: "gl tone-text", "Aa" } },
        NodeMark::Column(ColumnMark::Datetime) => rsx! { span { class: "gl tone-time", "◷" } },
        NodeMark::Column(ColumnMark::Enum) => rsx! { span { class: "gl tone-enum", "◐" } },
        NodeMark::Column(ColumnMark::Other) => rsx! { span { class: "gl tone-other", "•" } },
    }
}

#[component]
fn EditorPane(window_id: u64, view: ShellView) -> Element {
    let width = if view.panes.stack {
        "flex: 0 0 auto".to_string()
    } else {
        format!("width: {}px", view.editor_width)
    };
    let height = if view.panes.stack {
        format!("height: {}px", view.editor_height)
    } else {
        "flex: 1".to_string()
    };
    let style = format!("{width}; {height}; display: flex; flex-direction: column");
    rsx! {
        section { class: "editor-pane", style: "{style}",
            if let Some(notice_tab) = view.disk_notice.clone() {
                div { class: "banner",
                    "This file changed on disk."
                    button {
                        onclick: {
                            let notice_tab = notice_tab.clone();
                            move |_| runtime::apply(Message::ReloadDisk {
                                window: window_id,
                                tab: notice_tab.clone(),
                            })
                        },
                        "Reload"
                    }
                    button {
                        onclick: {
                            let notice_tab = notice_tab.clone();
                            move |_| runtime::apply(Message::DismissDisk {
                                window: window_id,
                                tab: notice_tab.clone(),
                            })
                        },
                        "Dismiss"
                    }
                }
            }
            if let Some(tab_id) = view.tab_id.clone() {
                SqlEditor { key: "{tab_id}-{view.revision}", window_id, tab_id, revision: view.revision }
            }
            if !view.diagnostics.is_empty() {
                div { class: "diagnostics",
                    for item in view.diagnostics.clone() {
                        DiagnosticRow { window_id, tab_id: view.tab_id.clone(), item }
                    }
                }
            }
        }
    }
}

#[component]
fn SqlEditor(window_id: u64, tab_id: String, revision: u64) -> Element {
    let _revision = revision;
    let initial = use_hook({
        let tab_id = tab_id.clone();
        move || editor_text(window_id, &tab_id)
    });
    let caret_tab = tab_id.clone();
    use_effect(move || {
        let tab_id = caret_tab.clone();
        spawn(async move {
            let (anchor, head) = editor_span(window_id, &tab_id);
            place_caret(anchor, head).await;
        });
    });
    let sql = editor_text(window_id, &tab_id);
    let pieces = highlight_sql(&sql);
    let ends_with_newline = sql.ends_with('\n');
    use_effect(use_reactive(&sql, |_| {
        spawn(async move {
            sync_highlight_scroll().await;
        });
    }));
    rsx! {
        div { class: "sql-editor",
            pre { id: "sql-highlight", aria_hidden: "true",
                for piece in pieces {
                    span { class: tone_class(piece.tone), "{piece.text}" }
                }
                if ends_with_newline {
                    br {}
                }
            }
            textarea {
                id: EDITOR_ID,
                spellcheck: "false",
                autocomplete: "off",
                autofocus: true,
                oninput: {
                    let tab_id = tab_id.clone();
                    move |event| {
                        runtime::apply(Message::Edit {
                            window: window_id,
                            tab: tab_id.clone(),
                            text: event.value(),
                        });
                    }
                },
                onscroll: move |_| {
                    spawn(async move {
                        sync_highlight_scroll().await;
                    });
                },
                onselect: {
                    let tab_id = tab_id.clone();
                    move |_| {
                        let tab_id = tab_id.clone();
                        spawn(async move {
                            sync_selection(window_id, tab_id).await;
                        });
                    }
                },
                "{initial}"
            }
        }
    }
}

fn tone_class(tone: SqlTone) -> &'static str {
    match tone {
        SqlTone::Plain => "",
        SqlTone::Keyword => "sql-kw",
        SqlTone::String => "sql-str",
        SqlTone::Number => "sql-num",
        SqlTone::Comment => "sql-com",
        SqlTone::Operator => "sql-op",
    }
}

#[component]
fn DiagnosticRow(window_id: u64, tab_id: Option<String>, item: DiagnosticView) -> Element {
    rsx! {
        div { class: "diagnostic {item.severity}",
            span { "{item.line}:{item.column} {item.message}" }
            for fix in item.fixes.clone() {
                button {
                    onclick: {
                        let tab_id = tab_id.clone();
                        let text = fix.text.clone();
                        let from = fix.from;
                        let to = fix.to;
                        move |_| {
                            let Some(tab_id) = tab_id.clone() else {
                                return;
                            };
                            runtime::apply(Message::ApplyFix {
                                window: window_id,
                                tab: tab_id,
                                from,
                                to,
                                text: text.clone(),
                            });
                        }
                    },
                    "{fix.name}"
                }
            }
        }
    }
}

#[component]
fn ResultsPane(window_id: u64, view: ShellView) -> Element {
    rsx! {
        section { class: "results",
            if view.results.is_empty() && view.restored {
                p { class: "placeholder", "Run to see results" }
            }
            for section in view.results.clone() {
                ResultBlock { window_id, tab_id: view.tab_id.clone(), section }
            }
            if view.more_rows {
                button {
                    onclick: {
                        let tab_id = view.tab_id.clone();
                        move |_| {
                            let Some(tab_id) = tab_id.clone() else {
                                return;
                            };
                            runtime::apply(Message::ShowMore { window: window_id, tab: tab_id });
                        }
                    },
                    "Show more"
                }
            }
        }
    }
}

#[component]
fn ResultBlock(window_id: u64, tab_id: Option<String>, section: SectionView) -> Element {
    rsx! {
        div { class: "section",
            if !section.sql.is_empty() {
                p { class: "muted", "{section.sql}" }
            }
            if let Some(error) = section.error.clone() {
                p { class: "error", "{error}" }
            } else if section.columns.is_empty() {
                p { class: "muted", "{section.summary}" }
            } else {
                table { class: "grid",
                    thead {
                        tr {
                            for column in section.columns.clone() {
                                th { style: "width: {column.width}px; min-width: {column.width}px",
                                    "{column.name}"
                                    span {
                                        class: "col-sash",
                                        onmousedown: {
                                            let tab_id = tab_id.clone();
                                            let name = column.name.clone();
                                            move |event| {
                                                let Some(tab_id) = tab_id.clone() else {
                                                    return;
                                                };
                                                start_drag(window_id, event, DragKind::Column {
                                                    tab: tab_id,
                                                    name: name.clone(),
                                                });
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    tbody {
                        for row in section.rows.clone() {
                            tr {
                                for cell in row {
                                    td {
                                        class: if cell.null { "null" } else if cell.numeric { "num" } else { "" },
                                        onclick: {
                                            let text = cell.text.clone();
                                            move |_| copy_cell(text.clone())
                                        },
                                        "{cell.text}"
                                    }
                                }
                            }
                        }
                    }
                }
                if section.truncated {
                    p { class: "muted", "Showing the first 1000 rows." }
                }
                if !section.summary.is_empty() {
                    p { class: "muted", "{section.summary}" }
                }
            }
        }
    }
}

#[component]
fn Footer(view: ShellView) -> Element {
    let summary = if !view.footer.is_empty() {
        view.footer.clone()
    } else {
        view.status.clone()
    };
    rsx! {
        footer { class: "footer",
            select {
                class: "theme",
                aria_label: "Theme",
                onchange: move |event| runtime::apply(Message::SetTheme(event.value())),
                for item in theme::THEMES {
                    option {
                        value: "{item.id}",
                        selected: view.theme == item.id,
                        "{item.name}"
                    }
                }
            }
            div { class: "summary", "{summary}" }
            div { class: "host", "{view.host_line}" }
        }
    }
}

#[component]
fn ModalLayer(view: ShellView) -> Element {
    rsx! {
        match view.modal.clone() {
            ModalView::None => rsx! {},
            ModalView::Close { many } => rsx! {
                div { class: "overlay",
                    div { class: "dialog narrow",
                        onclick: move |event| event.stop_propagation(),
                        p {
                            if many { "Save changes to the open files?" } else { "Save changes to this file?" }
                        }
                        div { class: "actions",
                            button {
                                onclick: move |_| runtime::apply(Message::CloseChoice { save: true }),
                                "Save"
                            }
                            button {
                                onclick: move |_| runtime::apply(Message::CloseChoice { save: false }),
                                "Don't Save"
                            }
                            button {
                                onclick: move |_| runtime::apply(Message::DismissModal),
                                "Cancel"
                            }
                        }
                    }
                }
            },
            ModalView::Connections(form) => rsx! {
                div {
                    class: "overlay",
                    onclick: move |_| runtime::apply(Message::DismissModal),
                    div {
                        class: "dialog",
                        onclick: move |event| event.stop_propagation(),
                        h2 { "Connections" }
                        div { class: "form",
                            div { class: "names",
                                button { onclick: move |_| runtime::apply(Message::FormNew), "New" }
                                for name in form.names.clone() {
                                    button {
                                        class: if form.editing.as_deref() == Some(name.as_str()) { "active" } else { "" },
                                        onclick: {
                                            let name = name.clone();
                                            move |_| runtime::apply(Message::FormPick(name.clone()))
                                        },
                                        "{name}"
                                    }
                                }
                            }
                            div { class: "fields",
                                FormField { key: "{form.generation}-name", generation: form.generation, field: Field::Name, label: "Name", secret: false }
                                FormField { key: "{form.generation}-host", generation: form.generation, field: Field::Host, label: "Host", secret: false }
                                FormField { key: "{form.generation}-port", generation: form.generation, field: Field::Port, label: "Port", secret: false }
                                FormField { key: "{form.generation}-user", generation: form.generation, field: Field::User, label: "User", secret: false }
                                FormField { key: "{form.generation}-password", generation: form.generation, field: Field::Password, label: "Password", secret: true }
                                FormField { key: "{form.generation}-schema", generation: form.generation, field: Field::Schema, label: "Schema", secret: false }
                                FormField { key: "{form.generation}-color", generation: form.generation, field: Field::Color, label: "Color", secret: false }
                                div { class: "checks",
                                    label {
                                        input {
                                            r#type: "checkbox",
                                            checked: form.ssl,
                                            onchange: move |event| runtime::apply(Message::FormFlag {
                                                ssl: Some(event.checked()),
                                                explain: None,
                                            }),
                                        }
                                        "SSL"
                                    }
                                    label {
                                        input {
                                            r#type: "checkbox",
                                            checked: form.explain_lint,
                                            onchange: move |event| runtime::apply(Message::FormFlag {
                                                ssl: None,
                                                explain: Some(event.checked()),
                                            }),
                                        }
                                        "Explain lint"
                                    }
                                }
                                if !form.status.is_empty() {
                                    p { class: "muted", "{form.status}" }
                                }
                            }
                        }
                        div { class: "actions",
                            button { onclick: move |_| runtime::apply(Message::FormTest), "Test" }
                            button { class: "primary", onclick: move |_| runtime::apply(Message::FormSave), "Save" }
                            button {
                                disabled: form.editing.is_none(),
                                onclick: move |_| runtime::apply(Message::FormDelete),
                                "Delete"
                            }
                            button { onclick: move |_| runtime::apply(Message::DismissModal), "Close" }
                        }
                    }
                }
            },
        }
    }
}

#[component]
fn FormField(generation: u64, field: Field, label: &'static str, secret: bool) -> Element {
    let _generation = generation;
    let initial = use_hook(move || form_value(field));
    let kind = if secret { "password" } else { "text" };
    rsx! {
        label { class: "field",
            span { "{label}" }
            input {
                r#type: "{kind}",
                spellcheck: "false",
                autocomplete: "off",
                value: "{initial}",
                oninput: move |event| {
                    runtime::apply(Message::FormInput {
                        field,
                        value: event.value(),
                    });
                },
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct ShellView {
    theme: String,
    file_menu: bool,
    dragging: bool,
    tabs: Vec<TabChipView>,
    tab_id: Option<String>,
    revision: u64,
    connection_names: Vec<String>,
    selected_connection: String,
    connection_missing: bool,
    badge_color: Option<String>,
    running: bool,
    layout: String,
    panes: Panes,
    schema_width: f32,
    editor_width: f32,
    editor_height: f32,
    schema_message: Option<String>,
    schema: Vec<SchemaNodeView>,
    disk_notice: Option<String>,
    diagnostics: Vec<DiagnosticView>,
    results: Vec<SectionView>,
    more_rows: bool,
    restored: bool,
    footer: String,
    status: String,
    host_line: String,
    modal: ModalView,
}

#[derive(Clone, PartialEq)]
struct TabChipView {
    index: usize,
    label: String,
    tooltip: String,
    active: bool,
    dirty: bool,
}

#[derive(Clone, PartialEq)]
struct SchemaNodeView {
    key: String,
    label: String,
    ident: Option<String>,
    depth: u8,
    open: bool,
    expandable: bool,
    mark: NodeMark,
    type_label: Option<String>,
    tone: String,
}

#[derive(Clone, Copy, PartialEq)]
enum NodeMark {
    Group,
    Table,
    Column(ColumnMark),
}

#[derive(Clone, PartialEq)]
struct DiagnosticView {
    severity: String,
    line: usize,
    column: usize,
    message: String,
    fixes: Vec<FixView>,
}

#[derive(Clone, PartialEq)]
struct FixView {
    name: String,
    from: usize,
    to: usize,
    text: String,
}

#[derive(Clone, PartialEq)]
struct SectionView {
    sql: String,
    error: Option<String>,
    summary: String,
    truncated: bool,
    columns: Vec<ColumnView>,
    rows: Vec<Vec<CellView>>,
}

#[derive(Clone, PartialEq)]
struct ColumnView {
    name: String,
    width: f64,
    numeric: bool,
}

#[derive(Clone, PartialEq)]
struct CellView {
    text: String,
    null: bool,
    numeric: bool,
}

#[derive(Clone, PartialEq)]
enum ModalView {
    None,
    Close { many: bool },
    Connections(ConnectionView),
}

#[derive(Clone, PartialEq)]
struct ConnectionView {
    generation: u64,
    names: Vec<String>,
    editing: Option<String>,
    ssl: bool,
    explain_lint: bool,
    status: String,
}

fn snapshot(window_id: u64) -> Option<ShellView> {
    let app = runtime::lock();
    let gui = app.window(window_id)?;
    let tab = gui.active_tab();
    let panes = logic::layout_panes(tab.map(|item| item.layout.as_str()).unwrap_or("all"));
    let selected = tab
        .and_then(|item| item.connection_name.clone())
        .unwrap_or_default();
    let connection = if selected.is_empty() {
        None
    } else {
        app.connection(&selected)
    };
    let connection_missing = !selected.is_empty() && connection.is_none();
    let badge_color = connection.map(|item| logic::badge_color(&item.name, item.color.as_deref()));
    let host_line = if let Some(item) = connection {
        let ping = match app.pings.get(&item.name) {
            Some(true) => "",
            Some(false) => " · offline",
            None => "",
        };
        format!("{} · {}:{}{ping}", item.name, item.host, item.port)
    } else if connection_missing {
        format!("{selected} · missing")
    } else {
        String::new()
    };
    let schema = connection
        .and_then(|item| app.schemas.get(&item.name))
        .cloned();
    let schema_message = if selected.is_empty() {
        Some("Choose a connection".into())
    } else if connection_missing {
        Some("Connection is missing".into())
    } else if let Some(error) = app.schema_errors.get(&selected) {
        Some(error.clone())
    } else if app.schema_loading.contains(&selected) && schema.is_none() {
        Some("Loading schema…".into())
    } else {
        None
    };
    let schema_nodes = schema
        .as_ref()
        .map(|snapshot| schema_nodes(snapshot, &gui.expanded))
        .unwrap_or_default();
    let sql = tab.map(crate::model::Tab::sql).unwrap_or_default();
    let diagnostics = tab
        .map(|item| {
            item.diagnostics
                .iter()
                .take(8)
                .map(|diagnostic| {
                    let (line, column) = logic::offset_to_position(&sql, diagnostic.from);
                    DiagnosticView {
                        severity: if diagnostic.severity == "error" {
                            "error".into()
                        } else {
                            "warn".into()
                        },
                        line: line + 1,
                        column: column + 1,
                        message: diagnostic.message.clone(),
                        fixes: diagnostic
                            .actions
                            .iter()
                            .map(|action| FixView {
                                name: action.name.clone(),
                                from: diagnostic.from,
                                to: diagnostic.to,
                                text: action.text.clone(),
                            })
                            .collect(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let show_rows = tab
        .map(|item| item.show_rows)
        .unwrap_or(logic::VISIBLE_ROWS);
    let widths = tab
        .map(|item| item.column_widths.clone())
        .unwrap_or_default();
    let results = tab
        .map(|item| {
            item.results
                .iter()
                .map(|section| section_view(section, show_rows, &widths))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let more_rows = tab.is_some_and(|item| {
        item.results
            .iter()
            .any(|section| section.rows.len() > item.show_rows)
    });
    let modal = match &app.modal {
        Modal::None => ModalView::None,
        Modal::Close(prompt) => ModalView::Close {
            many: prompt.quit || prompt.tab_ids.len() > 1,
        },
        Modal::Connections(form) => ModalView::Connections(ConnectionView {
            generation: app.form_generation,
            names: app.connection_names.clone(),
            editing: form.editing.clone(),
            ssl: form.ssl,
            explain_lint: form.explain_lint,
            status: form.status.clone(),
        }),
    };
    Some(ShellView {
        theme: app.theme_name.clone(),
        file_menu: gui.file_menu,
        dragging: app.drag.is_some(),
        tabs: gui
            .tabs
            .iter()
            .enumerate()
            .map(|(index, item)| TabChipView {
                index,
                label: item.label(),
                tooltip: logic::tab_tooltip(&item.sql(), item.file_path.as_deref()),
                active: index == gui.active,
                dirty: item.is_dirty(),
            })
            .collect(),
        tab_id: tab.map(|item| item.id.clone()),
        revision: tab.map(|item| item.buffer.revision).unwrap_or(0),
        connection_names: app.connection_names.clone(),
        selected_connection: selected,
        connection_missing,
        badge_color,
        running: tab.is_some_and(|item| item.running),
        layout: tab
            .map(|item| item.layout.clone())
            .unwrap_or_else(|| "all".into()),
        panes,
        schema_width: gui.schema_width,
        editor_width: gui.editor_width,
        editor_height: gui.editor_height,
        schema_message,
        schema: schema_nodes,
        disk_notice: tab
            .filter(|item| item.disk_notice)
            .map(|item| item.id.clone()),
        diagnostics,
        results,
        more_rows,
        restored: tab.is_some_and(|item| item.restored),
        footer: tab.map(|item| item.footer.clone()).unwrap_or_default(),
        status: app.status.clone(),
        host_line,
        modal,
    })
}

fn schema_nodes(
    snapshot: &SchemaSnapshot,
    expanded: &std::collections::HashSet<String>,
) -> Vec<SchemaNodeView> {
    let mut nodes = Vec::new();
    for schema in &snapshot.schemas {
        let schema_key = format!("schema:{}", schema.name);
        let schema_open = expanded.contains(&schema_key);
        nodes.push(SchemaNodeView {
            key: schema_key,
            label: schema.name.clone(),
            ident: Some(logic::quote_ident(&schema.name)),
            depth: 0,
            open: schema_open,
            expandable: true,
            mark: NodeMark::Group,
            type_label: None,
            tone: String::new(),
        });
        if !schema_open {
            continue;
        }
        for table in &schema.tables {
            let table_key = format!("table:{}:{}", schema.name, table.name);
            let table_open = expanded.contains(&table_key);
            nodes.push(SchemaNodeView {
                key: table_key,
                label: table.name.clone(),
                ident: Some(logic::quote_ident(&table.name)),
                depth: 1,
                open: table_open,
                expandable: true,
                mark: NodeMark::Table,
                type_label: None,
                tone: String::new(),
            });
            if !table_open {
                continue;
            }
            for column in &table.columns {
                let type_label = if column.column_type.is_empty() {
                    column.data_type.clone()
                } else {
                    column.column_type.clone()
                };
                nodes.push(SchemaNodeView {
                    key: format!("column:{}:{}:{}", schema.name, table.name, column.name),
                    label: column.name.clone(),
                    ident: Some(logic::quote_ident(&column.name)),
                    depth: 2,
                    open: false,
                    expandable: false,
                    mark: NodeMark::Column(logic::classify_column(
                        &column.column_key,
                        column.foreign_key,
                        &column.column_type,
                        &column.data_type,
                    )),
                    tone: logic::column_tone(&column.column_type, &column.data_type).into(),
                    type_label: if type_label.is_empty() {
                        None
                    } else {
                        Some(type_label)
                    },
                });
            }
        }
    }
    nodes
}

fn section_view(
    section: &ResultSection,
    show_rows: usize,
    widths: &std::collections::BTreeMap<String, f64>,
) -> SectionView {
    let columns = section
        .columns
        .iter()
        .map(|column| ColumnView {
            name: column.name.clone(),
            width: widths.get(&column.name).copied().unwrap_or(140.0),
            numeric: column.numeric,
        })
        .collect::<Vec<_>>();
    let rows = section
        .rows
        .iter()
        .take(show_rows)
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(index, cell)| {
                    let numeric = columns.get(index).is_some_and(|column| column.numeric);
                    match cell {
                        Some(text) => CellView {
                            text: text.clone(),
                            null: false,
                            numeric,
                        },
                        None => CellView {
                            text: "null".into(),
                            null: true,
                            numeric,
                        },
                    }
                })
                .collect()
        })
        .collect();
    let affected = section
        .rows_affected
        .map(|count| format!("{count} affected"))
        .unwrap_or_default();
    let elapsed = logic::format_elapsed(section.elapsed_ms);
    let summary = match (affected.is_empty(), elapsed.is_empty()) {
        (true, true) => String::new(),
        (false, true) => affected,
        (true, false) => elapsed,
        (false, false) => format!("{affected} · {elapsed}"),
    };
    SectionView {
        sql: logic::truncate(&logic::collapse(&section.sql), 180),
        error: section.error.clone(),
        summary,
        truncated: section.truncated,
        columns,
        rows,
    }
}

fn handle_key(window_id: u64, event: Event<KeyboardData>) {
    let key = event.key();
    if matches!(key, Key::Escape) {
        event.prevent_default();
        runtime::apply(Message::Escape(window_id));
        return;
    }
    let modifiers = event.modifiers();
    let command = modifiers.meta() || modifiers.ctrl();
    if !command || runtime::lock().blocks_shortcuts() {
        return;
    }
    let shift = modifiers.shift();
    let tab_id = {
        let app = runtime::lock();
        app.window(window_id)
            .and_then(|gui| gui.active_tab())
            .map(|tab| tab.id.clone())
    };
    let Some(letter) = character(&key) else {
        if matches!(key, Key::Enter) {
            event.prevent_default();
            let action = if shift {
                EditorAction::RunStatement
            } else {
                EditorAction::Run
            };
            queue_editor(window_id, tab_id, action);
        }
        return;
    };
    if matches!(letter, "a" | "c" | "x" | "v" | "A" | "C" | "X" | "V") {
        return;
    }
    let action = if letter.eq_ignore_ascii_case("t") && !shift {
        event.prevent_default();
        runtime::apply(Message::NewTab(window_id));
        return;
    } else if letter.eq_ignore_ascii_case("n") && !shift {
        event.prevent_default();
        runtime::apply(Message::NewWindow);
        return;
    } else if letter.eq_ignore_ascii_case("o") && !shift {
        event.prevent_default();
        runtime::apply(Message::Open(window_id));
        return;
    } else if letter.eq_ignore_ascii_case("s") && shift {
        event.prevent_default();
        EditorAction::SaveAs
    } else if letter.eq_ignore_ascii_case("s") {
        event.prevent_default();
        EditorAction::Save
    } else if letter.eq_ignore_ascii_case("w") && shift {
        event.prevent_default();
        runtime::apply(Message::CloseWindow(window_id));
        return;
    } else if letter.eq_ignore_ascii_case("w") {
        event.prevent_default();
        runtime::apply(Message::CloseTab(window_id));
        return;
    } else if letter.eq_ignore_ascii_case("q") {
        event.prevent_default();
        runtime::apply(Message::Quit);
        return;
    } else if letter.eq_ignore_ascii_case("z") && shift {
        event.prevent_default();
        let Some(tab_id) = tab_id else {
            return;
        };
        runtime::apply(Message::Redo {
            window: window_id,
            tab: tab_id,
        });
        return;
    } else if letter.eq_ignore_ascii_case("z") {
        event.prevent_default();
        let Some(tab_id) = tab_id else {
            return;
        };
        runtime::apply(Message::Undo {
            window: window_id,
            tab: tab_id,
        });
        return;
    } else if letter == "," && !shift {
        event.prevent_default();
        runtime::apply(Message::OpenConnections(window_id));
        return;
    } else {
        return;
    };
    queue_editor(window_id, tab_id, action);
}

fn character(key: &Key) -> Option<&str> {
    match key {
        Key::Character(value) => Some(value.as_str()),
        _ => None,
    }
}

fn queue_editor(window_id: u64, tab_id: Option<String>, action: EditorAction) {
    let Some(tab_id) = tab_id else {
        return;
    };
    spawn(async move {
        sync_selection(window_id, tab_id).await;
        let message = match action {
            EditorAction::Run => Message::Run {
                window: window_id,
                tab: None,
                statement: false,
            },
            EditorAction::RunStatement => Message::Run {
                window: window_id,
                tab: None,
                statement: true,
            },
            EditorAction::Save => Message::Save(window_id),
            EditorAction::SaveAs => Message::SaveAs(window_id),
            EditorAction::SaveSelection => Message::SaveSelection(window_id),
        };
        runtime::apply(message);
    });
}

fn close_menu_if_open(window_id: u64) {
    let open = runtime::lock()
        .window(window_id)
        .is_some_and(|gui| gui.file_menu);
    if open {
        runtime::apply(Message::ToggleFileMenu(window_id));
    }
}

fn drag_move(event: Event<MouseData>) {
    if runtime::lock().drag.is_none() {
        return;
    }
    let point = event.client_coordinates();
    runtime::apply(Message::DragMove {
        x: point.x as f32,
        y: point.y as f32,
    });
}

fn start_drag(window_id: u64, event: Event<MouseData>, kind: DragKind) {
    event.stop_propagation();
    let point = event.client_coordinates();
    runtime::apply(Message::StartDrag {
        window: window_id,
        kind,
        x: point.x as f32,
        y: point.y as f32,
    });
}

fn copy_cell(text: String) {
    let payload = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into());
    spawn(async move {
        let mut eval = document::eval(&format!(
            "navigator.clipboard.writeText({payload}).then(() => dioxus.send(true)).catch(() => dioxus.send(false))"
        ));
        if eval.recv::<bool>().await.unwrap_or(false) {
            runtime::apply(Message::Copied);
        }
    });
}

async fn sync_selection(window_id: u64, tab_id: String) {
    let mut eval = document::eval(
        r#"let node = document.getElementById("sql-editor");
if (!node) { dioxus.send([0, 0]); }
else { dioxus.send([node.selectionStart || 0, node.selectionEnd || 0]); }"#,
    );
    let Ok(value) = eval.recv::<serde_json::Value>().await else {
        return;
    };
    let Some(pair) = value.as_array() else {
        return;
    };
    let anchor = pair.first().and_then(|item| item.as_u64()).unwrap_or(0) as usize;
    let head = pair.get(1).and_then(|item| item.as_u64()).unwrap_or(0) as usize;
    runtime::apply(Message::Selection {
        window: window_id,
        tab: tab_id,
        anchor,
        head,
    });
}

async fn sync_highlight_scroll() {
    let mut eval = document::eval(
        r#"let area = document.getElementById("sql-editor");
let back = document.getElementById("sql-highlight");
if (area && back) {
  back.scrollTop = area.scrollTop;
  back.scrollLeft = area.scrollLeft;
}
dioxus.send(true);"#,
    );
    let _ = eval.recv::<bool>().await;
}

async fn place_caret(anchor: u32, head: u32) {
    let mut eval = document::eval(&format!(
        r#"let node = document.getElementById("sql-editor");
if (node) {{
  node.focus();
  node.setSelectionRange({anchor}, {head});
  dioxus.send(true);
}} else {{
  dioxus.send(false);
}}"#
    ));
    let _ = eval.recv::<bool>().await;
}

fn editor_text(window_id: u64, tab_id: &str) -> String {
    runtime::lock()
        .window(window_id)
        .and_then(|gui| gui.tabs.iter().find(|tab| tab.id == tab_id))
        .map(|tab| tab.sql())
        .unwrap_or_default()
}

fn editor_span(window_id: u64, tab_id: &str) -> (u32, u32) {
    let app = runtime::lock();
    let Some(tab) = app
        .window(window_id)
        .and_then(|gui| gui.tabs.iter().find(|tab| tab.id == tab_id))
    else {
        return (0, 0);
    };
    (
        logic::byte_to_utf16(&tab.buffer.text, tab.buffer.anchor),
        logic::byte_to_utf16(&tab.buffer.text, tab.buffer.head),
    )
}

fn form_value(field: Field) -> String {
    let app = runtime::lock();
    let Modal::Connections(form) = &app.modal else {
        return String::new();
    };
    match field {
        Field::Name => form.name.clone(),
        Field::Host => form.host.clone(),
        Field::Port => form.port.clone(),
        Field::User => form.user.clone(),
        Field::Password => form.password.clone(),
        Field::Schema => form.schema.clone(),
        Field::Color => form.color.clone(),
    }
}
