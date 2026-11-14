use dioxus::desktop::muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

pub fn app_menu() -> Menu {
    let menu = Menu::new();
    #[cfg(target_os = "macos")]
    {
        let application = application_menu();
        menu.append(&application).expect("app menu");
    }
    let file_menu = file_menu();
    let edit_menu = edit_menu();
    menu.append_items(&[&file_menu, &edit_menu])
        .expect("menu bar");
    menu
}

#[cfg(target_os = "macos")]
fn application_menu() -> Submenu {
    let menu = Submenu::new("sqlg", true);
    let connections = item("connections", "Connections…", Some("cmdOrCtrl+,"));
    let install = item("install-cli", "Install sqlg Command", None);
    let uninstall = item("uninstall-cli", "Uninstall sqlg Command", None);
    let quit = item("quit", "Quit", Some("cmdOrCtrl+Q"));
    let gap_a = PredefinedMenuItem::separator();
    let gap_b = PredefinedMenuItem::separator();
    menu.append_items(&[&connections, &gap_a, &install, &uninstall, &gap_b, &quit])
        .expect("app menu");
    menu
}

fn file_menu() -> Submenu {
    let menu = Submenu::new("File", true);
    let new_tab = item("new-tab", "New Tab", Some("cmdOrCtrl+T"));
    #[cfg(target_os = "macos")]
    let new_window = item("new-window", "New Window", Some("cmdOrCtrl+N"));
    #[cfg(not(target_os = "macos"))]
    let new_window = item("new-window", "New Window", None);
    let open = item("open", "Open…", Some("cmdOrCtrl+O"));
    let save = item("save", "Save", Some("cmdOrCtrl+S"));
    let save_as = item("save-as", "Save As…", Some("shift+cmdOrCtrl+S"));
    let save_selection = item("save-selection", "Save Selection As…", None);
    let close_tab = item("close-tab", "Close Tab", Some("cmdOrCtrl+W"));
    let close_window = item("close-window", "Close Window", Some("shift+cmdOrCtrl+W"));
    let gap_a = PredefinedMenuItem::separator();
    let gap_b = PredefinedMenuItem::separator();
    menu.append_items(&[
        &new_tab,
        &new_window,
        &open,
        &gap_a,
        &save,
        &save_as,
        &save_selection,
        &gap_b,
        &close_tab,
        &close_window,
    ])
    .expect("file menu");
    #[cfg(not(target_os = "macos"))]
    {
        let connections = item("connections", "Connections…", Some("cmdOrCtrl+,"));
        let install = item("install-cli", "Install sqlg Command", None);
        let uninstall = item("uninstall-cli", "Uninstall sqlg Command", None);
        let quit = item("quit", "Quit", Some("cmdOrCtrl+Q"));
        let gap_c = PredefinedMenuItem::separator();
        let gap_d = PredefinedMenuItem::separator();
        menu.append_items(&[&gap_c, &connections, &install, &uninstall, &gap_d, &quit])
            .expect("file menu");
    }
    menu
}

fn edit_menu() -> Submenu {
    let menu = Submenu::new("Edit", true);
    let cut = PredefinedMenuItem::cut(None);
    let copy = PredefinedMenuItem::copy(None);
    let paste = PredefinedMenuItem::paste(None);
    let gap = PredefinedMenuItem::separator();
    let select_all = PredefinedMenuItem::select_all(None);
    menu.append_items(&[&cut, &copy, &paste, &gap, &select_all])
        .expect("edit menu");
    menu
}

fn item(id: &str, label: &str, accelerator: Option<&str>) -> MenuItem {
    let accelerator = accelerator.map(|value| value.parse().expect("menu accelerator"));
    MenuItem::with_id(id, label, true, accelerator)
}
