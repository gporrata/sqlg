use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::Duration;

use dioxus::desktop::tao::dpi::{LogicalPosition, LogicalSize};
use dioxus::desktop::tao::event::{Event, WindowEvent};
use dioxus::desktop::tao::window::{Fullscreen, Theme, WindowBuilder};
use dioxus::desktop::{Config, DesktopContext, WindowCloseBehaviour};
use dioxus::dioxus_core::spawn_forever;
use dioxus::prelude::{VirtualDom, spawn};
use futures::StreamExt;
use sqlg_mysql::QueryEvent;

use crate::ipc::Primary;
use crate::message::{Effect, ExplainJob, LintJob, Message, QueryJob};
use crate::model::App;
use crate::update;
use crate::view::{self, HEAD};

static APP: OnceLock<Mutex<App>> = OnceLock::new();
static EFFECTS: Mutex<Vec<Effect>> = Mutex::new(Vec::new());
static INBOX: Mutex<Vec<Mail>> = Mutex::new(Vec::new());
static CHILD_IDS: Mutex<VecDeque<u64>> = Mutex::new(VecDeque::new());
static CLOSING: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
static EPOCH: AtomicU64 = AtomicU64::new(1);
static ROOT: AtomicU64 = AtomicU64::new(0);
static SERVICE_STARTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static DESKTOPS: std::cell::RefCell<std::collections::HashMap<u64, DesktopContext>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

enum Mail {
    Show(u64),
    Message(Message),
}

struct WindowGeometry {
    title: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    fullscreen: bool,
    dark: bool,
}

pub fn launch(primary: Primary) {
    let mut app = App::boot(primary);
    let effects = update::startup(&mut app);
    let geometry = geometry_of(&app, app.root_id);
    ROOT.store(app.root_id, Ordering::Relaxed);
    let _ = APP.set(Mutex::new(app));
    enqueue(effects);
    dioxus::LaunchBuilder::desktop()
        .with_cfg(window_config(&geometry))
        .launch(view::app_root);
}

pub fn root_id() -> u64 {
    ROOT.load(Ordering::Relaxed)
}

pub fn epoch() -> u64 {
    EPOCH.load(Ordering::Relaxed)
}

pub fn take_child_id() -> u64 {
    CHILD_IDS
        .lock()
        .expect("child ids")
        .pop_front()
        .unwrap_or(0)
}

pub fn lock() -> std::sync::MutexGuard<'static, App> {
    APP.get().expect("app").lock().expect("app lock")
}

pub fn apply(message: Message) {
    let effects = {
        let mut app = lock();
        update::update(&mut app, message)
    };
    enqueue(effects);
    EPOCH.fetch_add(1, Ordering::Relaxed);
}

pub fn attach_window(window_id: u64) {
    let desktop = dioxus::desktop::window();
    desktop.set_close_behavior(WindowCloseBehaviour::WindowHides);
    DESKTOPS.with(|slots| {
        slots.borrow_mut().insert(window_id, desktop.clone());
    });
    let tao_id = desktop.id();
    let scale_source = desktop.clone();
    desktop.create_wry_event_handler(move |event, _| {
        let Event::WindowEvent {
            window_id: event_window,
            event,
            ..
        } = event
        else {
            return;
        };
        if *event_window != tao_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if closing(window_id) {
                    return;
                }
                post(Mail::Show(window_id));
                post(Mail::Message(Message::CloseWindow(window_id)));
            }
            WindowEvent::Focused(true) => post(Mail::Message(Message::Focused(window_id))),
            WindowEvent::Moved(position) => {
                let scale = scale_source.scale_factor().max(1.0);
                post(Mail::Message(Message::Moved {
                    window: window_id,
                    x: position.x as f64 / scale,
                    y: position.y as f64 / scale,
                }));
            }
            WindowEvent::Resized(size) => {
                let scale = scale_source.scale_factor().max(1.0);
                post(Mail::Message(Message::Resized {
                    window: window_id,
                    width: size.width as f64 / scale,
                    height: size.height as f64 / scale,
                }));
            }
            WindowEvent::DroppedFile(path) => {
                post(Mail::Message(Message::Drop {
                    window: window_id,
                    path: path.clone(),
                }));
            }
            _ => {}
        }
    });
    if SERVICE_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        spawn_forever(async {
            service_loop().await;
        });
    }
    sync_chrome(window_id);
}

pub fn sync_chrome(window_id: u64) {
    let (title, dark) = {
        let app = lock();
        (
            app.title(window_id),
            crate::theme::find_theme(&app.theme_name).dark,
        )
    };
    let Some(desktop) = desktop_for(window_id) else {
        return;
    };
    desktop.set_title(&title);
    desktop.set_theme(Some(if dark { Theme::Dark } else { Theme::Light }));
}

fn service_loop() -> impl std::future::Future<Output = ()> {
    async {
        drain_effects().await;
        let mut frames = 0u32;
        loop {
            tokio::time::sleep(Duration::from_millis(32)).await;
            for mail in take_mail() {
                match mail {
                    Mail::Show(id) => show_window(id),
                    Mail::Message(message) => apply(message),
                }
            }
            frames = frames.wrapping_add(1);
            if frames % 8 == 0 {
                apply(Message::Tick);
            }
            drain_effects().await;
        }
    }
}

async fn drain_effects() {
    loop {
        let batch = {
            let mut queued = EFFECTS.lock().expect("effects");
            if queued.is_empty() {
                return;
            }
            std::mem::take(&mut *queued)
        };
        for effect in batch {
            run_effect(effect).await;
        }
    }
}

async fn run_effect(effect: Effect) {
    match effect {
        Effect::Query(job) => start_query(job),
        Effect::Cancel(run_id) => {
            let db = lock().db.clone();
            spawn(async move {
                db.cancel_run(&run_id).await;
            });
        }
        Effect::Schema(name) => {
            let db = lock().db.clone();
            spawn(async move {
                let result = match db.pool_for(&name).await {
                    Ok(pool) => sqlg_mysql::load_schema(&pool).await,
                    Err(err) => Err(err),
                };
                apply(Message::SchemaLoaded { name, result });
            });
        }
        Effect::Ping(name) => {
            let db = lock().db.clone();
            spawn(async move {
                let ok = match db.pool_for(&name).await {
                    Ok(pool) => sqlx::query("select 1").execute(&pool).await.is_ok(),
                    Err(_) => false,
                };
                apply(Message::Pinged { name, ok });
            });
        }
        Effect::Lint(job) => start_lint(job),
        Effect::Explain(job) => start_explain(job),
        Effect::Test {
            connection,
            password,
        } => {
            spawn(async move {
                let result = crate::db::Db::test_connection(&connection, &password).await;
                apply(Message::FormTested(result));
            });
        }
        Effect::Pick {
            window,
            tab,
            purpose,
            save,
        } => {
            let picked = pick_path(save).await;
            apply(Message::FilePicked {
                window,
                tab,
                purpose,
                path: picked,
            });
        }
        Effect::OpenWindow(id) => open_native(id).await,
        Effect::Focus(id) => {
            if let Some(desktop) = desktop_for(id) {
                desktop.set_focus();
            }
        }
        Effect::Close(id) => close_native(id),
        Effect::Exit => std::process::exit(0),
        Effect::Install => {
            spawn(async move {
                let result = tokio::task::spawn_blocking(crate::files::install_cli)
                    .await
                    .unwrap_or_else(|err| Err(err.to_string()));
                apply(Message::CliResult(result));
            });
        }
        Effect::Uninstall => {
            spawn(async move {
                let result = tokio::task::spawn_blocking(crate::files::uninstall_cli)
                    .await
                    .unwrap_or_else(|err| Err(err.to_string()));
                apply(Message::CliResult(result));
            });
        }
    }
}

fn start_query(job: QueryJob) {
    let db = lock().db.clone();
    let QueryJob {
        window,
        tab,
        run_id,
        sql,
        connection,
        cancel,
        connection_id,
    } = job;
    let consumer_tab = tab.clone();
    let consumer_run = run_id.clone();
    spawn(async move {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let producer = async move {
            let pool = match db.pool_for(&connection).await {
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
                    db.forget_run(&run_id);
                    return;
                }
            };
            let finished = run_id.clone();
            sqlg_mysql::execute_script(pool, run_id, sql, cancel, connection_id, move |event| {
                let _ = tx.unbounded_send(event);
            })
            .await;
            db.forget_run(&finished);
        };
        let consumer = async move {
            while let Some(event) = rx.next().await {
                apply(Message::Query {
                    window,
                    tab: consumer_tab.clone(),
                    run_id: consumer_run.clone(),
                    event,
                });
            }
        };
        let _ = futures::future::join(producer, consumer).await;
    });
}

fn start_lint(job: LintJob) {
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let LintJob {
            window,
            tab,
            generation,
            sql,
            catalog,
        } = job;
        let diagnostics = tokio::task::spawn_blocking(move || sqlg_core::lint_sql(&sql, &catalog))
            .await
            .unwrap_or_default();
        apply(Message::LintReady {
            window,
            tab,
            generation,
            diagnostics,
        });
    });
}

fn start_explain(job: ExplainJob) {
    let db = lock().db.clone();
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(750)).await;
        let result = db.explain(&job.name, &job.sql).await;
        apply(Message::ExplainReady {
            window: job.window,
            tab: job.tab,
            generation: job.generation,
            result,
        });
    });
}

async fn pick_path(save: bool) -> Option<std::path::PathBuf> {
    let dialog = rfd::AsyncFileDialog::new().add_filter("SQL", &["sql"]);
    let handle = if save {
        dialog.save_file().await
    } else {
        dialog.pick_file().await
    };
    handle.map(|file| file.path().to_path_buf())
}

async fn open_native(id: u64) {
    let geometry = {
        let app = lock();
        geometry_of(&app, id)
    };
    let Some(anchor) = any_desktop() else {
        return;
    };
    CHILD_IDS.lock().expect("child ids").push_back(id);
    let pending = anchor.new_window(VirtualDom::new(view::child_root), window_config(&geometry));
    let _desktop = pending.await;
}

fn close_native(id: u64) {
    CLOSING.lock().expect("closing").insert(id);
    if let Some(desktop) = desktop_for(id) {
        desktop.set_close_behavior(WindowCloseBehaviour::WindowCloses);
        desktop.close();
    }
    DESKTOPS.with(|slots| {
        slots.borrow_mut().remove(&id);
    });
}

fn show_window(id: u64) {
    if let Some(desktop) = desktop_for(id) {
        desktop.set_visible(true);
        desktop.set_focus();
    }
}

fn desktop_for(id: u64) -> Option<DesktopContext> {
    DESKTOPS.with(|slots| slots.borrow().get(&id).cloned())
}

fn any_desktop() -> Option<DesktopContext> {
    DESKTOPS.with(|slots| slots.borrow().values().next().cloned())
}

fn closing(id: u64) -> bool {
    CLOSING.lock().expect("closing").contains(&id)
}

fn post(mail: Mail) {
    INBOX.lock().expect("inbox").push(mail);
}

fn take_mail() -> Vec<Mail> {
    std::mem::take(&mut *INBOX.lock().expect("inbox"))
}

fn enqueue(effects: Vec<Effect>) {
    EFFECTS.lock().expect("effects").extend(effects);
}

fn geometry_of(app: &App, id: u64) -> WindowGeometry {
    let gui = app.window(id);
    WindowGeometry {
        title: app.title(id),
        x: gui.map(|window| window.x).unwrap_or(80.0),
        y: gui.map(|window| window.y).unwrap_or(80.0),
        width: gui.map(|window| window.width).unwrap_or(1200.0),
        height: gui.map(|window| window.height).unwrap_or(800.0),
        fullscreen: gui.is_some_and(|window| window.fullscreen),
        dark: crate::theme::find_theme(&app.theme_name).dark,
    }
}

fn window_config(geometry: &WindowGeometry) -> Config {
    let mut builder = WindowBuilder::new()
        .with_title(&geometry.title)
        .with_inner_size(LogicalSize::new(
            geometry.width.max(720.0),
            geometry.height.max(480.0),
        ))
        .with_min_inner_size(LogicalSize::new(720.0, 480.0))
        .with_position(LogicalPosition::new(geometry.x, geometry.y))
        .with_always_on_top(false);
    if geometry.fullscreen {
        builder = builder.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    let background = if geometry.dark {
        (30, 30, 30, 255)
    } else {
        (244, 246, 247, 255)
    };
    Config::new()
        .with_window(builder)
        .with_menu(None)
        .with_close_behaviour(WindowCloseBehaviour::WindowHides)
        .with_custom_head(HEAD.to_string())
        .with_background_color(background)
        .with_disable_context_menu(true)
}
