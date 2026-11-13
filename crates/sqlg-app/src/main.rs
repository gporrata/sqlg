mod db;
mod files;
mod ipc;
mod logic;
mod message;
mod state;
mod update;
mod view;

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use iced::event::{self, Event};
use iced::keyboard::{self, Key};
use iced::{Subscription, Task, Theme, window};

use crate::message::Message;
use crate::state::App;

static PRIMARY: OnceLock<Mutex<Option<ipc::Primary>>> = OnceLock::new();

fn main() -> iced::Result {
    match ipc::acquire(&sqlg_core::data_dir()) {
        ipc::Acquire::Forwarded => Ok(()),
        ipc::Acquire::Primary(primary) => {
            let _ = PRIMARY.set(Mutex::new(Some(primary)));
            iced::daemon(boot, update::update, view::view)
                .title(title)
                .subscription(subscription)
                .theme(theme)
                .run()
        }
    }
}

fn boot() -> (App, Task<Message>) {
    let primary = PRIMARY
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|mut slot| slot.take())
        .expect("gui lock");
    let mut app = App::boot(primary);
    let task = update::open_restored(&mut app);
    (app, task)
}

fn title(app: &App, id: window::Id) -> String {
    app.window(id)
        .and_then(|gui| gui.active_tab())
        .map(|tab| format!("sqlg — {}", tab.label()))
        .unwrap_or_else(|| "sqlg".into())
}

fn theme(app: &App, _id: window::Id) -> Theme {
    if app.theme_name == "dark" {
        Theme::Dark
    } else {
        Theme::Light
    }
}

fn subscription(_app: &App) -> Subscription<Message> {
    Subscription::batch([
        event::listen_with(listen),
        iced::time::every(Duration::from_millis(250)).map(|_| Message::Tick),
    ])
}

fn listen(event: Event, _status: event::Status, id: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            repeat,
            ..
        }) => {
            if repeat {
                return None;
            }
            if matches!(key, Key::Named(keyboard::key::Named::Escape)) {
                return Some(Message::Escape(id));
            }
            if !modifiers.command() {
                return None;
            }
            if matches!(key.as_ref(), Key::Named(keyboard::key::Named::Enter)) {
                return Some(Message::Run {
                    window: id,
                    tab: None,
                    statement: modifiers.shift(),
                });
            }
            match key.to_latin(physical_key) {
                Some('t') | Some('T') => Some(Message::NewTab(id)),
                Some('n') | Some('N') => Some(Message::NewWindow),
                Some('o') | Some('O') => Some(Message::Open(id)),
                Some('s') | Some('S') if modifiers.shift() => Some(Message::SaveAs(id)),
                Some('s') | Some('S') => Some(Message::Save(id)),
                Some('w') | Some('W') if modifiers.shift() => Some(Message::CloseWindow(id)),
                Some('w') | Some('W') => Some(Message::CloseTab(id)),
                Some('q') | Some('Q') => Some(Message::Quit),
                _ if comma_key(&key) => Some(Message::OpenConnections(id)),
                _ => None,
            }
        }
        Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => Some(Message::DragEnd),
        Event::Window(window::Event::CloseRequested) => Some(Message::CloseWindow(id)),
        Event::Window(window::Event::Closed) => Some(Message::WindowClosed(id)),
        Event::Window(window::Event::Moved(point)) => Some(Message::Moved { window: id, point }),
        Event::Window(window::Event::Resized(size)) => Some(Message::Resized { window: id, size }),
        Event::Window(window::Event::Opened { position, size }) => Some(Message::WindowOpened {
            window: id,
            position,
            size,
        }),
        Event::Window(window::Event::Focused) => Some(Message::Focused(id)),
        Event::Window(window::Event::FileDropped(path)) => Some(Message::Drop { window: id, path }),
        _ => None,
    }
}

fn comma_key(key: &Key) -> bool {
    matches!(key.as_ref(), Key::Character(","))
}
