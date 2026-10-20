use sqlg_core::WindowSnapshot;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Theme, TitleBarStyle, WebviewUrl};

pub fn create_window(app: &AppHandle, label: &str) -> Result<(), String> {
    if app.get_webview_window(label).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("sqlg")
        .inner_size(1440.0, 900.0)
        .min_inner_size(960.0, 640.0)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(LogicalPosition::new(14.0, 18.0))
        .accept_first_mouse(true)
        .build()
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub fn restore(app: &AppHandle, windows: &[WindowSnapshot], theme: &str) -> Result<(), String> {
    if windows.is_empty() {
        create_window(app, "main")?;
    }
    for snapshot in windows {
        create_window(app, &snapshot.label)?;
        place(app, snapshot)?;
    }
    apply_theme(app, theme);
    Ok(())
}

pub fn place(app: &AppHandle, snapshot: &WindowSnapshot) -> Result<(), String> {
    let Some(window) = app.get_webview_window(&snapshot.label) else {
        return Ok(());
    };
    let (x, y) = clamp(app, snapshot.x, snapshot.y, snapshot.width, snapshot.height);
    let _ = window.set_position(LogicalPosition::new(x, y));
    let _ = window.set_size(LogicalSize::new(
        snapshot.width.max(960.0),
        snapshot.height.max(640.0),
    ));
    if snapshot.fullscreen {
        let _ = window.set_fullscreen(true);
    }
    Ok(())
}

pub fn apply_theme(app: &AppHandle, theme: &str) {
    // Native chrome is only light or dark. CSS carries the full palette.
    // Keep the light ids aligned with src/app/themes.ts.
    let native = if is_light_theme(theme) {
        Theme::Light
    } else {
        Theme::Dark
    };
    for (_, window) in app.webview_windows() {
        let _ = window.set_theme(Some(native));
    }
}

pub fn is_light_theme(theme: &str) -> bool {
    matches!(theme, "light" | "github-light" | "solarized-light")
}

pub fn normalize_theme(theme: &str) -> String {
    let theme = theme.trim();
    let ok = !theme.is_empty()
        && theme.len() <= 64
        && !theme.starts_with('-')
        && !theme.ends_with('-')
        && theme
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if ok {
        theme.to_string()
    } else {
        "light".to_string()
    }
}

pub fn clamp(app: &AppHandle, x: f64, y: f64, width: f64, height: f64) -> (f64, f64) {
    let Ok(monitors) = app.available_monitors() else {
        return (x, y);
    };
    if monitors.is_empty() {
        return (x, y);
    }
    let visible = monitors.iter().any(|monitor| {
        let scale = monitor.scale_factor();
        let origin = monitor.position();
        let size = monitor.size();
        let left = origin.x as f64 / scale;
        let top = origin.y as f64 / scale;
        let right = left + size.width as f64 / scale;
        let bottom = top + size.height as f64 / scale;
        let cx = x + width.min(80.0) / 2.0;
        let cy = y + 20.0;
        cx >= left && cx <= right && cy >= top && cy <= bottom && height > 0.0
    });
    if visible {
        (x, y)
    } else if let Some(primary) = monitors.first() {
        let scale = primary.scale_factor();
        let origin = primary.position();
        (
            origin.x as f64 / scale + 40.0,
            origin.y as f64 / scale + 40.0,
        )
    } else {
        (x, y)
    }
}

pub fn front_label(app: &AppHandle) -> String {
    let windows = app.webview_windows();
    if let Some((label, _)) = windows
        .iter()
        .find(|(_, window)| window.is_focused().unwrap_or(false))
    {
        return label.clone();
    }
    if windows.contains_key("main") {
        return "main".into();
    }
    windows
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| "main".into())
}
