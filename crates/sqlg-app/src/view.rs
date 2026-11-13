use iced::widget::text::Wrapping;
use iced::widget::text_editor::{Binding, KeyPress};
use iced::widget::{
    button, checkbox, column, container, mouse_area, pick_list, row, scrollable, space, stack,
    text, text_editor, text_input, tooltip,
};
use iced::{Background, Color, Element, Font, Length, Theme, alignment, window};

use crate::logic;
use crate::message::{DragKind, Field, Message};
use crate::state::{App, ConnForm, GuiWindow, Modal, ResultSection, Tab};

pub fn view(app: &App, window_id: window::Id) -> Element<'_, Message> {
    let Some(gui) = app.window(window_id) else {
        return text("sqlg").into();
    };
    let dark = app.theme_name == "dark";
    let body = window_body(app, gui, dark);
    let mut layers: Vec<Element<'_, Message>> = vec![body];
    if gui.file_menu {
        layers.push(file_menu(window_id));
    }
    if let Some(modal) = modal_view(app, gui, dark) {
        layers.push(modal);
    }
    if app
        .drag
        .as_ref()
        .is_some_and(|drag| drag.window == window_id)
    {
        layers.push(
            mouse_area(space().width(Length::Fill).height(Length::Fill))
                .on_move(Message::DragMove)
                .on_release(Message::DragEnd)
                .into(),
        );
    }
    stack(layers).into()
}

fn window_body<'a>(app: &'a App, gui: &'a GuiWindow, dark: bool) -> Element<'a, Message> {
    let Some(tab) = gui.active_tab() else {
        return text("sqlg").into();
    };
    let mut column_items: Vec<Element<'a, Message>> = vec![toolbar(app, gui, tab)];
    if tab.disk_notice {
        column_items.push(disk_banner(gui.id, &tab.id));
    }
    column_items.push(workspace(app, gui, tab, dark));
    column_items.push(footer(app, tab));
    container(column(column_items).spacing(6))
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn toolbar<'a>(app: &'a App, gui: &'a GuiWindow, tab: &'a Tab) -> Element<'a, Message> {
    let mut tabs: Vec<Element<'a, Message>> = Vec::new();
    for (index, item) in gui.tabs.iter().enumerate() {
        let label = if item.is_dirty() {
            format!("● {}", item.label())
        } else {
            item.label()
        };
        let mut title: Vec<Element<'a, Message>> = Vec::new();
        if let Some(connection) = item
            .connection_name
            .as_deref()
            .and_then(|name| app.connection(name))
        {
            title.push(badge(&connection.name, connection.color.as_deref()));
        }
        title.push(text(label).size(13).into());
        let mut control = button(row(title).spacing(6).align_y(alignment::Vertical::Center))
            .padding([4, 8])
            .on_press(Message::SelectTab {
                window: gui.id,
                index,
            });
        if index == gui.active {
            control = control.style(button::primary);
        } else {
            control = control.style(button::secondary);
        }
        let tip = logic::tab_tooltip(&item.sql(), item.file_path.as_deref());
        tabs.push(
            tooltip(
                control,
                container(text(tip).size(12)).padding(6),
                tooltip::Position::Bottom,
            )
            .into(),
        );
    }
    let selected = tab
        .content
        .selection()
        .is_some_and(|value| !value.trim().is_empty());
    let run_label = if selected { "Run Selection" } else { "Run" };
    let tab_id = tab.id.clone();
    let window_id = gui.id;
    let mut tools: Vec<Element<'a, Message>> = vec![
        button(text("File").size(13))
            .padding([4, 8])
            .on_press(Message::ToggleFileMenu(window_id))
            .into(),
        scrollable(row(tabs).spacing(4).align_y(alignment::Vertical::Center))
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            ))
            .width(Length::Fill)
            .into(),
    ];
    let names = app.connection_names.as_slice();
    let current = tab.connection_name.clone();
    let pick_tab = tab.id.clone();
    tools.push(
        pick_list(names, current, move |name| Message::SetConnection {
            window: window_id,
            tab: pick_tab.clone(),
            name,
        })
        .placeholder("Connection")
        .width(150)
        .into(),
    );
    if let Some(name) = tab.connection_name.as_deref() {
        let ok = app.pings.get(name).copied();
        tools.push(ping_dot(ok));
    }
    for (layout, label) in [
        ("all", "All"),
        ("editor-results", "Split"),
        ("below", "Below"),
        ("below-only", "Stack"),
        ("editor", "Editor"),
        ("results", "Results"),
    ] {
        let mut control =
            button(text(label).size(12))
                .padding([4, 6])
                .on_press(Message::SetLayout {
                    window: window_id,
                    layout: layout.into(),
                });
        if tab.layout == layout {
            control = control.style(button::primary);
        } else {
            control = control.style(button::text);
        }
        tools.push(control.into());
    }
    tools.push(
        button(
            text(if app.theme_name == "dark" {
                "Dark"
            } else {
                "Light"
            })
            .size(12),
        )
        .padding([4, 6])
        .on_press(Message::ToggleTheme)
        .into(),
    );
    tools.push(
        button(text(run_label).size(13))
            .padding([4, 8])
            .style(button::primary)
            .on_press_maybe(if tab.running {
                None
            } else {
                Some(Message::Run {
                    window: window_id,
                    tab: Some(tab_id.clone()),
                    statement: false,
                })
            })
            .into(),
    );
    if tab.running {
        tools.push(
            button(text("Cancel").size(13))
                .padding([4, 8])
                .on_press(Message::Cancel {
                    window: window_id,
                    tab: tab_id,
                })
                .into(),
        );
    }
    scrollable(row(tools).spacing(6).align_y(alignment::Vertical::Center))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .into()
}

fn workspace<'a>(
    app: &'a App,
    gui: &'a GuiWindow,
    tab: &'a Tab,
    dark: bool,
) -> Element<'a, Message> {
    let panes = logic::layout_panes(&tab.layout);
    let mut parts: Vec<Element<'a, Message>> = Vec::new();
    if panes.schema {
        parts.push(
            container(schema_pane(app, gui, tab))
                .width(gui.schema_width)
                .height(Length::Fill)
                .into(),
        );
        parts.push(vertical_sash(Message::StartDrag {
            window: gui.id,
            kind: DragKind::Schema,
        }));
    }
    if panes.stack {
        let mut stacked: Vec<Element<'a, Message>> = Vec::new();
        if panes.editor {
            stacked.push(
                container(editor_pane(gui, tab, dark))
                    .height(gui.editor_height)
                    .width(Length::Fill)
                    .into(),
            );
            if panes.results {
                stacked.push(horizontal_sash(Message::StartDrag {
                    window: gui.id,
                    kind: DragKind::EditorHeight,
                }));
            }
        }
        if panes.results {
            stacked.push(results_pane(gui, tab, dark));
        }
        parts.push(
            column(stacked)
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        );
    } else {
        if panes.editor {
            let editor = container(editor_pane(gui, tab, dark)).height(Length::Fill);
            let editor = if panes.results {
                editor.width(gui.editor_width)
            } else {
                editor.width(Length::Fill)
            };
            parts.push(editor.into());
            if panes.results {
                parts.push(vertical_sash(Message::StartDrag {
                    window: gui.id,
                    kind: DragKind::EditorWidth,
                }));
            }
        }
        if panes.results {
            parts.push(results_pane(gui, tab, dark));
        }
    }
    row(parts).spacing(0).height(Length::Fill).into()
}

fn editor_pane<'a>(gui: &'a GuiWindow, tab: &'a Tab, dark: bool) -> Element<'a, Message> {
    let window_id = gui.id;
    let tab_edit = tab.id.clone();
    let tab_keys = tab.id.clone();
    let highlight = if dark {
        iced::highlighter::Theme::SolarizedDark
    } else {
        iced::highlighter::Theme::InspiredGitHub
    };
    let editor = text_editor(&tab.content)
        .placeholder("New query")
        .font(Font::MONOSPACE)
        .size(14)
        .padding(8)
        .wrapping(Wrapping::None)
        .height(Length::Fill)
        .on_action(move |action| Message::Edit {
            window: window_id,
            tab: tab_edit.clone(),
            action,
        })
        .key_binding(move |press| editor_binding(window_id, &tab_keys, press))
        .highlight("sql", highlight);
    let mut items = vec![editor.into()];
    if !tab.diagnostics.is_empty() {
        items.push(diagnostics(gui.id, tab));
    }
    column(items).spacing(4).height(Length::Fill).into()
}

fn editor_binding(window: window::Id, tab: &str, press: KeyPress) -> Option<Binding<Message>> {
    if press.modifiers.command() {
        if matches!(
            press.key.as_ref(),
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
        ) {
            return Some(Binding::Custom(Message::Run {
                window,
                tab: Some(tab.to_string()),
                statement: press.modifiers.shift(),
            }));
        }
        match press.key.to_latin(press.physical_key) {
            Some('c') | Some('x') | Some('v') | Some('a') => {
                return Binding::<Message>::from_key_press(press);
            }
            Some('z') if press.modifiers.shift() => {
                return Some(Binding::Custom(Message::Redo {
                    window,
                    tab: tab.to_string(),
                }));
            }
            Some('z') => {
                return Some(Binding::Custom(Message::Undo {
                    window,
                    tab: tab.to_string(),
                }));
            }
            _ => {
                if matches!(press.key.as_ref(), iced::keyboard::Key::Named(_)) {
                    return Binding::<Message>::from_key_press(press);
                }
                return None;
            }
        }
    }
    Binding::<Message>::from_key_press(press)
}

fn diagnostics<'a>(window: window::Id, tab: &'a Tab) -> Element<'a, Message> {
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    for diagnostic in tab.diagnostics.iter().take(8) {
        let mut line: Vec<Element<'a, Message>> =
            vec![text(diagnostic.message.clone()).size(12).into()];
        for action in &diagnostic.actions {
            line.push(
                button(text(action.name.clone()).size(12))
                    .padding([2, 6])
                    .on_press(Message::ApplyFix {
                        window,
                        tab: tab.id.clone(),
                        from: diagnostic.from,
                        to: diagnostic.to,
                        text: action.text.clone(),
                    })
                    .into(),
            );
        }
        rows.push(row(line).spacing(6).into());
    }
    scrollable(column(rows).spacing(2))
        .height(Length::Shrink)
        .into()
}

fn schema_pane<'a>(app: &'a App, gui: &'a GuiWindow, tab: &'a Tab) -> Element<'a, Message> {
    let Some(name) = tab.connection_name.as_deref() else {
        return text("Choose a connection").size(13).into();
    };
    if let Some(err) = app.schema_errors.get(name) {
        return column(vec![
            text(err.clone()).size(12).into(),
            button(text("Reload schema").size(12))
                .on_press(Message::ReloadSchema { window: gui.id })
                .into(),
        ])
        .spacing(6)
        .into();
    }
    let Some(snapshot) = app.schemas.get(name) else {
        let label = if app.schema_loading.contains(name) {
            "Loading schema…"
        } else {
            "Schema not loaded"
        };
        return column(vec![
            text(label).size(13).into(),
            button(text("Load schema").size(12))
                .on_press(Message::ReloadSchema { window: gui.id })
                .into(),
        ])
        .spacing(6)
        .into();
    };
    let mut rows: Vec<Element<'a, Message>> = vec![
        button(text("Reload schema").size(12))
            .on_press(Message::ReloadSchema { window: gui.id })
            .into(),
    ];
    for schema in &snapshot.schemas {
        let open = gui.expanded.contains(&schema.name);
        rows.push(schema_row(
            gui.id,
            &tab.id,
            &schema.name,
            open,
            Message::ToggleExpand {
                window: gui.id,
                key: schema.name.clone(),
            },
            Message::InsertText {
                window: gui.id,
                tab: tab.id.clone(),
                text: logic::quote_ident(&schema.name),
            },
        ));
        if !open {
            continue;
        }
        for table in &schema.tables {
            let key = format!("{}\u{0}{}", schema.name, table.name);
            let table_open = gui.expanded.contains(&key);
            let qualified = format!(
                "{}.{}",
                logic::quote_ident(&schema.name),
                logic::quote_ident(&table.name)
            );
            rows.push(schema_row(
                gui.id,
                &tab.id,
                &format!("  {}", table.name),
                table_open,
                Message::ToggleExpand {
                    window: gui.id,
                    key: key.clone(),
                },
                Message::InsertText {
                    window: gui.id,
                    tab: tab.id.clone(),
                    text: qualified,
                },
            ));
            if !table_open {
                continue;
            }
            for column in &table.columns {
                let kind = if column.column_key.eq_ignore_ascii_case("pri") {
                    "pk"
                } else if column.foreign_key {
                    "fk"
                } else {
                    column.data_type.as_str()
                };
                rows.push(
                    button(
                        text(format!("    {}  {kind}", column.name))
                            .size(12)
                            .font(Font::MONOSPACE),
                    )
                    .padding([1, 4])
                    .style(button::text)
                    .on_press(Message::InsertText {
                        window: gui.id,
                        tab: tab.id.clone(),
                        text: logic::quote_ident(&column.name),
                    })
                    .into(),
                );
            }
        }
    }
    scrollable(column(rows).spacing(1))
        .height(Length::Fill)
        .into()
}

fn schema_row<'a>(
    _window: window::Id,
    _tab: &str,
    label: &str,
    open: bool,
    toggle: Message,
    insert: Message,
) -> Element<'a, Message> {
    let marker = if open { "▾" } else { "▸" };
    mouse_area(
        button(
            text(format!("{marker} {label}"))
                .size(13)
                .font(Font::MONOSPACE),
        )
        .padding([1, 4])
        .style(button::text)
        .on_press(toggle),
    )
    .on_double_click(insert)
    .into()
}

fn results_pane<'a>(gui: &'a GuiWindow, tab: &'a Tab, dark: bool) -> Element<'a, Message> {
    if tab.results.is_empty() {
        let message = if tab.running {
            "Running…"
        } else {
            "Run to see results"
        };
        return container(text(message).size(14))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .into();
    }
    let mut sections: Vec<Element<'a, Message>> = Vec::new();
    for (index, section) in tab.results.iter().enumerate() {
        sections.push(result_section(gui, tab, index, section, dark));
    }
    sections.push(text("Click a cell to copy it").size(11).into());
    scrollable(column(sections).spacing(8))
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn result_section<'a>(
    gui: &'a GuiWindow,
    tab: &'a Tab,
    index: usize,
    section: &'a ResultSection,
    dark: bool,
) -> Element<'a, Message> {
    let snippet = sqlg_core::snippet(&section.sql, 72);
    let meta = if let Some(message) = &section.error {
        message.clone()
    } else if section.columns.is_empty() {
        match section.rows_affected {
            Some(rows) => format!("{rows} rows affected"),
            None => "running".into(),
        }
    } else {
        let elapsed = logic::format_elapsed(section.elapsed_ms);
        let mut text = format!("{} rows", section.rows.len());
        if !elapsed.is_empty() {
            text = format!("{text} · {elapsed}");
        }
        if section.truncated {
            text = format!("{text} · showing first {} rows", sqlg_mysql::ROW_CAP);
        }
        text
    };
    let mut items = vec![
        text(format!("{}. {snippet}", index + 1))
            .font(Font::MONOSPACE)
            .size(12)
            .into(),
        text(meta).size(12).into(),
    ];
    if section.error.is_some() {
        return column(items).spacing(2).into();
    }
    if !section.columns.is_empty() {
        items.push(result_grid(gui, tab, section, dark));
        if section.rows.len() > tab.show_rows {
            let remaining = section.rows.len() - tab.show_rows;
            items.push(
                button(text(format!("Show {remaining} more")).size(12))
                    .on_press(Message::ShowMore {
                        window: gui.id,
                        tab: tab.id.clone(),
                    })
                    .into(),
            );
        }
    }
    column(items).spacing(4).into()
}

fn result_grid<'a>(
    gui: &'a GuiWindow,
    tab: &'a Tab,
    section: &'a ResultSection,
    dark: bool,
) -> Element<'a, Message> {
    let limit = tab.show_rows.min(section.rows.len());
    let mut grid: Vec<Element<'a, Message>> = vec![grid_row(
        gui,
        tab,
        section
            .columns
            .iter()
            .map(|column| (column.name.clone(), column.name.clone(), false, false))
            .collect(),
        true,
        dark,
    )];
    for row_values in section.rows.iter().take(limit) {
        let cells = section
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let value = row_values.get(index).cloned().flatten();
                let null = value.is_none();
                (
                    column.name.clone(),
                    value.unwrap_or_else(|| "null".into()),
                    column.numeric,
                    null,
                )
            })
            .collect();
        grid.push(grid_row(gui, tab, cells, false, dark));
    }
    scrollable(column(grid).spacing(0))
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .height(Length::Fill)
        .into()
}

fn grid_row<'a>(
    gui: &'a GuiWindow,
    tab: &'a Tab,
    cells: Vec<(String, String, bool, bool)>,
    header: bool,
    dark: bool,
) -> Element<'a, Message> {
    let mut items: Vec<Element<'a, Message>> = Vec::new();
    for (name, value, numeric, null) in cells {
        let width = tab
            .column_widths
            .get(&name)
            .copied()
            .unwrap_or(140.0)
            .clamp(logic::COLUMN_MIN, 800.0);
        let mut label = text(value.clone()).size(12).font(Font::MONOSPACE);
        if header {
            label = label.color(Color::from_rgb8(0x5d, 0x6d, 0x7e));
        } else if null {
            label = label.color(Color::from_rgba(0.45, 0.45, 0.45, 1.0));
        } else if numeric {
            label = label.color(if dark {
                Color::from_rgb8(0x85, 0xc1, 0xe9)
            } else {
                Color::from_rgb8(0x1a, 0x52, 0x7a)
            });
        }
        let cell = container(label).width(width as f32).padding([2, 4]);
        if header {
            items.push(
                mouse_area(cell)
                    .on_press(Message::StartDrag {
                        window: gui.id,
                        kind: DragKind::Column {
                            tab: tab.id.clone(),
                            name,
                        },
                    })
                    .into(),
            );
        } else {
            items.push(mouse_area(cell).on_press(Message::Copy(value)).into());
        }
    }
    row(items).spacing(0).into()
}

fn footer<'a>(app: &'a App, tab: &'a Tab) -> Element<'a, Message> {
    row(vec![
        text(tab.footer.clone()).size(12).into(),
        space().width(Length::Fill).into(),
        text(app.status.clone()).size(12).into(),
    ])
    .align_y(alignment::Vertical::Center)
    .into()
}

fn disk_banner<'a>(window: window::Id, tab: &str) -> Element<'a, Message> {
    let tab_reload = tab.to_string();
    let tab_keep = tab.to_string();
    row(vec![
        text("This file changed on disk.").size(13).into(),
        button(text("Reload").size(12))
            .on_press(Message::ReloadDisk {
                window,
                tab: tab_reload,
            })
            .into(),
        button(text("Keep").size(12))
            .on_press(Message::DismissDisk {
                window,
                tab: tab_keep,
            })
            .into(),
    ])
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn file_menu(window: window::Id) -> Element<'static, Message> {
    let item = |label: &'static str, message: Message| {
        button(text(label).size(13))
            .width(Length::Fill)
            .padding([4, 8])
            .style(button::text)
            .on_press(message)
    };
    container(
        column(vec![
            item("New Tab", Message::NewTab(window)).into(),
            item("New Window", Message::NewWindow).into(),
            item("Open…", Message::Open(window)).into(),
            item("Save", Message::Save(window)).into(),
            item("Save As…", Message::SaveAs(window)).into(),
            item("Save Selection As…", Message::SaveSelection(window)).into(),
            item("Close Tab", Message::CloseTab(window)).into(),
            item("Close Window", Message::CloseWindow(window)).into(),
            item("Connections…", Message::OpenConnections(window)).into(),
            item("Install sqlg Command", Message::InstallCli).into(),
            item("Uninstall sqlg Command", Message::UninstallCli).into(),
            item("Quit", Message::Quit).into(),
        ])
        .spacing(2),
    )
    .padding(8)
    .width(220)
    .style(menu_style)
    .into()
}

fn modal_view<'a>(app: &'a App, gui: &'a GuiWindow, dark: bool) -> Option<Element<'a, Message>> {
    match &app.modal {
        Modal::None => None,
        Modal::Connections(form) => Some(overlay(connections_modal(app, form), dark)),
        Modal::Close(prompt) if prompt.quit || prompt.window == gui.id => {
            let message = if prompt.tab_ids.len() > 1 {
                "Save changes to the open files?"
            } else {
                "Save changes to this file?"
            };
            Some(overlay(close_modal(message), dark))
        }
        Modal::Close(_) => None,
    }
}

fn overlay<'a>(card: Element<'a, Message>, dark: bool) -> Element<'a, Message> {
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(Color::from_rgba(
                0.0,
                0.0,
                0.0,
                if dark { 0.45 } else { 0.25 },
            ))),
            ..Default::default()
        })
        .into()
}

fn connections_modal<'a>(app: &'a App, form: &'a ConnForm) -> Element<'a, Message> {
    let mut listed: Vec<Element<'a, Message>> = vec![
        button(text("New").size(13))
            .width(Length::Fill)
            .on_press(Message::FormNew)
            .into(),
    ];
    for connection in &app.connections {
        let name = connection.name.clone();
        listed.push(
            button(text(name.clone()).size(13))
                .width(Length::Fill)
                .style(
                    if form.editing.as_deref() == Some(connection.name.as_str()) {
                        button::primary
                    } else {
                        button::text
                    },
                )
                .on_press(Message::FormPick(name))
                .into(),
        );
    }
    let fields = column(vec![
        labeled("Name", Field::Name, &form.name, false),
        labeled("Host", Field::Host, &form.host, false),
        labeled("Port", Field::Port, &form.port, false),
        labeled("User", Field::User, &form.user, false),
        labeled("Password", Field::Password, &form.password, true),
        labeled("Schema", Field::Schema, &form.schema, false),
        labeled("Color", Field::Color, &form.color, false),
        checkbox(form.ssl)
            .label("SSL")
            .on_toggle(|value| Message::FormFlag {
                ssl: Some(value),
                explain: None,
            })
            .into(),
        checkbox(form.explain_lint)
            .label("Explain lint")
            .on_toggle(|value| Message::FormFlag {
                ssl: None,
                explain: Some(value),
            })
            .into(),
        text(form.status.clone()).size(12).into(),
        row(vec![
            button(text("Test").size(13))
                .on_press(Message::FormTest)
                .into(),
            button(text("Save").size(13))
                .style(button::primary)
                .on_press(Message::FormSave)
                .into(),
            button(text("Delete").size(13))
                .on_press_maybe(form.editing.is_some().then_some(Message::FormDelete))
                .into(),
            button(text("Close").size(13))
                .on_press(Message::DismissModal)
                .into(),
        ])
        .spacing(8)
        .into(),
    ])
    .spacing(6);
    container(
        row(vec![
            scrollable(column(listed).spacing(4))
                .height(Length::Fill)
                .width(180)
                .into(),
            fields.width(Length::Fill).into(),
        ])
        .spacing(12),
    )
    .padding(16)
    .width(680)
    .height(460)
    .style(menu_style)
    .into()
}

fn labeled<'a>(label: &'a str, field: Field, value: &'a str, secure: bool) -> Element<'a, Message> {
    column(vec![
        text(label).size(12).into(),
        text_input(label, value)
            .secure(secure)
            .on_input(move |value| Message::FormInput { field, value })
            .into(),
    ])
    .spacing(2)
    .into()
}

fn close_modal<'a>(message: &'a str) -> Element<'a, Message> {
    container(
        column(vec![
            text(message).size(16).into(),
            row(vec![
                button(text("Save").size(13))
                    .style(button::primary)
                    .on_press(Message::CloseChoice { save: true })
                    .into(),
                button(text("Don't Save").size(13))
                    .on_press(Message::CloseChoice { save: false })
                    .into(),
                button(text("Cancel").size(13))
                    .on_press(Message::DismissModal)
                    .into(),
            ])
            .spacing(8)
            .into(),
        ])
        .spacing(12),
    )
    .padding(16)
    .width(420)
    .style(menu_style)
    .into()
}

fn badge<'a>(name: &'a str, color: Option<&'a str>) -> Element<'a, Message> {
    let paint = logic::badge_color(name, color);
    container(text(name).size(11).color(Color::WHITE))
        .padding([2, 6])
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(paint)),
            text_color: Some(Color::WHITE),
            ..Default::default()
        })
        .into()
}

fn ping_dot<'a>(ok: Option<bool>) -> Element<'a, Message> {
    let paint = match ok {
        Some(true) => Color::from_rgb8(0x1e, 0x84, 0x49),
        Some(false) => Color::from_rgb8(0xc0, 0x39, 0x2b),
        None => Color::from_rgb8(0x99, 0x99, 0x99),
    };
    container(space().width(8).height(8))
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(paint)),
            ..Default::default()
        })
        .into()
}

fn vertical_sash(message: Message) -> Element<'static, Message> {
    mouse_area(container(space().width(6).height(Length::Fill)).style(sash_style))
        .on_press(message)
        .into()
}

fn horizontal_sash(message: Message) -> Element<'static, Message> {
    mouse_area(container(space().width(Length::Fill).height(6)).style(sash_style))
        .on_press(message)
        .into()
}

fn menu_style(theme: &Theme) -> container::Style {
    let dark = matches!(theme, Theme::Dark);
    let background = if dark {
        Color::from_rgb8(0x2b, 0x2b, 0x2b)
    } else {
        Color::from_rgb8(0xf4, 0xf6, 0xf7)
    };
    let border = if dark {
        Color::from_rgb8(0x55, 0x55, 0x55)
    } else {
        Color::from_rgb8(0xbd, 0xc3, 0xc7)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: iced::Border {
            width: 1.0,
            color: border,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn sash_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xbd, 0xc3, 0xc7))),
        ..Default::default()
    }
}
