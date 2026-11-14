pub struct AppTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    pub bg: &'static str,
    pub panel: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub line: &'static str,
    pub accent: &'static str,
    pub accent_ink: &'static str,
    pub danger: &'static str,
    pub editor: &'static str,
    pub chip: &'static str,
    pub warn: &'static str,
}

pub const THEMES: &[AppTheme] = &[
    AppTheme {
        id: "light",
        name: "Light",
        dark: false,
        bg: "#ffffff",
        panel: "#f5f5f7",
        ink: "#1d1d1f",
        muted: "#666666",
        line: "#d0d0d4",
        accent: "#0a64d8",
        accent_ink: "#ffffff",
        danger: "#c41a16",
        editor: "#ffffff",
        chip: "#e8e8ea",
        warn: "#6a4c00",
    },
    AppTheme {
        id: "dark",
        name: "Dark",
        dark: true,
        bg: "#1e1e1e",
        panel: "#252526",
        ink: "#e5e5e7",
        muted: "#a0a0a5",
        line: "#3a3a3c",
        accent: "#4077c2",
        accent_ink: "#ffffff",
        danger: "#f48771",
        editor: "#1e1e1e",
        chip: "#2b2b2d",
        warn: "#d7ba7d",
    },
    AppTheme {
        id: "github-light",
        name: "GitHub Light",
        dark: false,
        bg: "#ffffff",
        panel: "#f6f8fa",
        ink: "#1f2328",
        muted: "#59636e",
        line: "#d0d7de",
        accent: "#0969da",
        accent_ink: "#ffffff",
        danger: "#cf222e",
        editor: "#ffffff",
        chip: "#eef5fc",
        warn: "#9a6700",
    },
    AppTheme {
        id: "github-dark",
        name: "GitHub Dark",
        dark: true,
        bg: "#0d1117",
        panel: "#010409",
        ink: "#e6edf3",
        muted: "#8b949e",
        line: "#30363d",
        accent: "#1f6feb",
        accent_ink: "#ffffff",
        danger: "#ff7b72",
        editor: "#0d1117",
        chip: "#161b22",
        warn: "#d29922",
    },
    AppTheme {
        id: "one-dark",
        name: "One Dark",
        dark: true,
        bg: "#282c34",
        panel: "#21252b",
        ink: "#abb2bf",
        muted: "#5c6370",
        line: "#3e4452",
        accent: "#61afef",
        accent_ink: "#282c34",
        danger: "#e06c75",
        editor: "#282c34",
        chip: "#2c313c",
        warn: "#d19a66",
    },
    AppTheme {
        id: "one-dark-pro",
        name: "One Dark Pro",
        dark: true,
        bg: "#282c34",
        panel: "#21252b",
        ink: "#abb2bf",
        muted: "#5c6370",
        line: "#3e4452",
        accent: "#528bff",
        accent_ink: "#282c34",
        danger: "#c24038",
        editor: "#282c34",
        chip: "#2c313a",
        warn: "#d19a66",
    },
    AppTheme {
        id: "one-dark-pro-night-flat",
        name: "One Dark Pro Night Flat",
        dark: true,
        bg: "#16191d",
        panel: "#16191d",
        ink: "#abb2bf",
        muted: "#667187",
        line: "#3e4452",
        accent: "#528bff",
        accent_ink: "#16191d",
        danger: "#c24038",
        editor: "#16191d",
        chip: "#23272e",
        warn: "#d19a66",
    },
    AppTheme {
        id: "ayu-dark",
        name: "Ayu Dark",
        dark: true,
        bg: "#10141c",
        panel: "#0d1017",
        ink: "#fcf8ecbd",
        muted: "#5a6673",
        line: "#1b1f29",
        accent: "#e6b450",
        accent_ink: "#765b24",
        danger: "#d95757",
        editor: "#10141c",
        chip: "#141821",
        warn: "#e6b450",
    },
    AppTheme {
        id: "ayu-mirage",
        name: "Ayu Mirage",
        dark: true,
        bg: "#242936",
        panel: "#1f2430",
        ink: "#fefaecc4",
        muted: "#6e7c8f",
        line: "#171b24",
        accent: "#ffcc66",
        accent_ink: "#735923",
        danger: "#ff6666",
        editor: "#242936",
        chip: "#282e3b",
        warn: "#ffcc66",
    },
    AppTheme {
        id: "ayu-light",
        name: "Ayu Light",
        dark: false,
        bg: "#fcfcfc",
        panel: "#f8f9fa",
        ink: "#020a12a3",
        muted: "#adaeb1",
        line: "#e3e6e8",
        accent: "#f29718",
        accent_ink: "#7e4b01",
        danger: "#e65050",
        editor: "#fcfcfc",
        chip: "#ebeef0",
        warn: "#f29718",
    },
    AppTheme {
        id: "dracula",
        name: "Dracula",
        dark: true,
        bg: "#282a36",
        panel: "#21222c",
        ink: "#f8f8f2",
        muted: "#6272a4",
        line: "#44475a",
        accent: "#bd93f9",
        accent_ink: "#282a36",
        danger: "#ff5555",
        editor: "#282a36",
        chip: "#343746",
        warn: "#ffb86c",
    },
    AppTheme {
        id: "nord",
        name: "Nord",
        dark: true,
        bg: "#2e3440",
        panel: "#3b4252",
        ink: "#d8dee9",
        muted: "#7b88a1",
        line: "#4c566a",
        accent: "#88c0d0",
        accent_ink: "#2e3440",
        danger: "#bf616a",
        editor: "#2e3440",
        chip: "#434c5e",
        warn: "#ebcb8b",
    },
    AppTheme {
        id: "solarized-light",
        name: "Solarized Light",
        dark: false,
        bg: "#fdf6e3",
        panel: "#eee8d5",
        ink: "#586e75",
        muted: "#657b83",
        line: "#d6d0bc",
        accent: "#268bd2",
        accent_ink: "#ffffff",
        danger: "#dc322f",
        editor: "#fdf6e3",
        chip: "#eee8d5",
        warn: "#b58900",
    },
    AppTheme {
        id: "solarized-dark",
        name: "Solarized Dark",
        dark: true,
        bg: "#002b36",
        panel: "#073642",
        ink: "#93a1a1",
        muted: "#839496",
        line: "#094552",
        accent: "#268bd2",
        accent_ink: "#ffffff",
        danger: "#dc322f",
        editor: "#002b36",
        chip: "#073642",
        warn: "#b58900",
    },
    AppTheme {
        id: "monokai",
        name: "Monokai",
        dark: true,
        bg: "#272822",
        panel: "#1e1f1c",
        ink: "#f8f8f2",
        muted: "#90886f",
        line: "#3e3d32",
        accent: "#a6e22e",
        accent_ink: "#272822",
        danger: "#f92672",
        editor: "#272822",
        chip: "#3e3d32",
        warn: "#e6db74",
    },
    AppTheme {
        id: "tokyo-night",
        name: "Tokyo Night",
        dark: true,
        bg: "#1a1b26",
        panel: "#16161e",
        ink: "#c0caf5",
        muted: "#737aa2",
        line: "#292e42",
        accent: "#7aa2f7",
        accent_ink: "#1a1b26",
        danger: "#f7768e",
        editor: "#1a1b26",
        chip: "#222433",
        warn: "#e0af68",
    },
    AppTheme {
        id: "gruvbox-dark",
        name: "Gruvbox Dark",
        dark: true,
        bg: "#282828",
        panel: "#282828",
        ink: "#ebdbb2",
        muted: "#a89984",
        line: "#3c3836",
        accent: "#fe8019",
        accent_ink: "#282828",
        danger: "#fb4934",
        editor: "#282828",
        chip: "#3c3836",
        warn: "#d79921",
    },
    AppTheme {
        id: "gruvbox-light",
        name: "Gruvbox Light",
        dark: false,
        bg: "#fbf1c7",
        panel: "#fbf1c7",
        ink: "#3c3836",
        muted: "#7c6f64",
        line: "#d5c4a1",
        accent: "#af3a03",
        accent_ink: "#fbf1c7",
        danger: "#9d0006",
        editor: "#fbf1c7",
        chip: "#ebdbb2",
        warn: "#b57614",
    },
];

pub fn find_theme(id: &str) -> &'static AppTheme {
    THEMES
        .iter()
        .find(|theme| theme.id == id)
        .unwrap_or(&THEMES[0])
}

pub fn css_vars(theme: &AppTheme) -> String {
    format!(
        "--bg:{bg};--panel:{panel};--ink:{ink};--muted:{muted};--line:{line};--accent:{accent};--accent-ink:{accent_ink};--danger:{danger};--editor:{editor};--chip:{chip};--warn:{warn}",
        bg = theme.bg,
        panel = theme.panel,
        ink = theme.ink,
        muted = theme.muted,
        line = theme.line,
        accent = theme.accent,
        accent_ink = theme.accent_ink,
        danger = theme.danger,
        editor = theme.editor,
        chip = theme.chip,
        warn = theme.warn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_theme_falls_back_to_light() {
        assert_eq!(find_theme("").id, "light");
        assert_eq!(find_theme("missing").name, "Light");
        assert!(!find_theme("light").dark);
        assert!(find_theme("dark").dark);
    }

    #[test]
    fn common_themes_use_their_vscode_colors() {
        let gruvbox = find_theme("gruvbox-dark");
        assert_eq!(gruvbox.editor, "#282828");
        assert_eq!(gruvbox.ink, "#ebdbb2");
        assert_eq!(gruvbox.accent, "#fe8019");
        let ayu = find_theme("ayu-dark");
        assert!(ayu.dark);
        assert_eq!(ayu.editor, "#10141c");
        assert_eq!(ayu.accent, "#e6b450");
        assert_eq!(ayu.danger, "#d95757");
        assert!(!find_theme("ayu-light").dark);
        assert_eq!(find_theme("ayu-mirage").panel, "#1f2430");
        let dark_pro = find_theme("one-dark-pro");
        assert_eq!(dark_pro.name, "One Dark Pro");
        assert_eq!(dark_pro.editor, "#282c34");
        assert_eq!(dark_pro.panel, "#21252b");
        assert_eq!(dark_pro.accent, "#528bff");
        assert!(css_vars(ayu).contains("--editor:#10141c"));
        let mut ids = THEMES.iter().map(|theme| theme.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), THEMES.len());
    }
}
