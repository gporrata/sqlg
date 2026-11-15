use crate::color::WebColor;
use crate::webcolor;

pub struct AppTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,
    pub bg: WebColor,
    pub panel: WebColor,
    pub ink: WebColor,
    pub muted: WebColor,
    pub line: WebColor,
    pub accent: WebColor,
    pub accent_ink: WebColor,
    pub danger: WebColor,
    pub editor: WebColor,
    pub chip: WebColor,
    pub warn: WebColor,
    pub keyword: WebColor,
    pub operator: WebColor,
    pub string: WebColor,
    pub number: WebColor,
    pub comment: WebColor,
}

pub const THEMES: &[AppTheme] = &[
    AppTheme {
        id: "light",
        name: "Light",
        dark: false,
        bg: webcolor!("#ffffff"),
        panel: webcolor!("#f5f5f7"),
        ink: webcolor!("#1d1d1f"),
        muted: webcolor!("#666666"),
        line: webcolor!("#d0d0d4"),
        accent: webcolor!("#0a64d8"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#c41a16"),
        editor: webcolor!("#ffffff"),
        chip: webcolor!("#e8e8ea"),
        warn: webcolor!("#6a4c00"),
        keyword: webcolor!("#ad3da4"),
        operator: webcolor!("#ad3da4"),
        string: webcolor!("#c41a16"),
        number: webcolor!("#1c00cf"),
        comment: webcolor!("#6e6e73"),
    },
    AppTheme {
        id: "dark",
        name: "Dark",
        dark: true,
        bg: webcolor!("#1e1e1e"),
        panel: webcolor!("#252526"),
        ink: webcolor!("#e5e5e7"),
        muted: webcolor!("#a0a0a5"),
        line: webcolor!("#3a3a3c"),
        accent: webcolor!("#4077c2"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#f48771"),
        editor: webcolor!("#1e1e1e"),
        chip: webcolor!("#2b2b2d"),
        warn: webcolor!("#d7ba7d"),
        keyword: webcolor!("#ff7ab2"),
        operator: webcolor!("#ff7ab2"),
        string: webcolor!("#ff8170"),
        number: webcolor!("#d9c97c"),
        comment: webcolor!("#8b98a5"),
    },
    AppTheme {
        id: "github-light",
        name: "GitHub Light",
        dark: false,
        bg: webcolor!("#ffffff"),
        panel: webcolor!("#f6f8fa"),
        ink: webcolor!("#1f2328"),
        muted: webcolor!("#59636e"),
        line: webcolor!("#d0d7de"),
        accent: webcolor!("#0969da"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#cf222e"),
        editor: webcolor!("#ffffff"),
        chip: webcolor!("#eef5fc"),
        warn: webcolor!("#9a6700"),
        keyword: webcolor!("#cf222e"),
        operator: webcolor!("#cf222e"),
        string: webcolor!("#0a3069"),
        number: webcolor!("#0550ae"),
        comment: webcolor!("#59636e"),
    },
    AppTheme {
        id: "github-dark",
        name: "GitHub Dark",
        dark: true,
        bg: webcolor!("#0d1117"),
        panel: webcolor!("#010409"),
        ink: webcolor!("#e6edf3"),
        muted: webcolor!("#8b949e"),
        line: webcolor!("#30363d"),
        accent: webcolor!("#1f6feb"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#ff7b72"),
        editor: webcolor!("#0d1117"),
        chip: webcolor!("#161b22"),
        warn: webcolor!("#d29922"),
        keyword: webcolor!("#ff7b72"),
        operator: webcolor!("#ff7b72"),
        string: webcolor!("#a5d6ff"),
        number: webcolor!("#79c0ff"),
        comment: webcolor!("#8b949e"),
    },
    AppTheme {
        id: "one-dark",
        name: "One Dark",
        dark: true,
        bg: webcolor!("#282c34"),
        panel: webcolor!("#21252b"),
        ink: webcolor!("#abb2bf"),
        muted: webcolor!("#5c6370"),
        line: webcolor!("#3e4452"),
        accent: webcolor!("#61afef"),
        accent_ink: webcolor!("#282c34"),
        danger: webcolor!("#e06c75"),
        editor: webcolor!("#282c34"),
        chip: webcolor!("#2c313c"),
        warn: webcolor!("#d19a66"),
        keyword: webcolor!("#c678dd"),
        operator: webcolor!("#56b6c2"),
        string: webcolor!("#98c379"),
        number: webcolor!("#d19a66"),
        comment: webcolor!("#5c6370"),
    },
    AppTheme {
        id: "one-dark-pro",
        name: "One Dark Pro",
        dark: true,
        bg: webcolor!("#282c34"),
        panel: webcolor!("#21252b"),
        ink: webcolor!("#abb2bf"),
        muted: webcolor!("#5c6370"),
        line: webcolor!("#3e4452"),
        accent: webcolor!("#528bff"),
        accent_ink: webcolor!("#282c34"),
        danger: webcolor!("#c24038"),
        editor: webcolor!("#282c34"),
        chip: webcolor!("#2c313a"),
        warn: webcolor!("#d19a66"),
        keyword: webcolor!("#c678dd"),
        operator: webcolor!("#56b6c2"),
        string: webcolor!("#98c379"),
        number: webcolor!("#d19a66"),
        comment: webcolor!("#5c6370"),
    },
    AppTheme {
        id: "one-dark-pro-night-flat",
        name: "One Dark Pro Night Flat",
        dark: true,
        bg: webcolor!("#16191d"),
        panel: webcolor!("#16191d"),
        ink: webcolor!("#abb2bf"),
        muted: webcolor!("#667187"),
        line: webcolor!("#3e4452"),
        accent: webcolor!("#528bff"),
        accent_ink: webcolor!("#16191d"),
        danger: webcolor!("#c24038"),
        editor: webcolor!("#16191d"),
        chip: webcolor!("#23272e"),
        warn: webcolor!("#d19a66"),
        keyword: webcolor!("#c678dd"),
        operator: webcolor!("#56b6c2"),
        string: webcolor!("#98c379"),
        number: webcolor!("#d19a66"),
        comment: webcolor!("#5c6370"),
    },
    AppTheme {
        id: "ayu-dark",
        name: "Ayu Dark",
        dark: true,
        bg: webcolor!("#10141c"),
        panel: webcolor!("#0d1017"),
        ink: webcolor!("#fcf8ecbd"),
        muted: webcolor!("#5a6673"),
        line: webcolor!("#1b1f29"),
        accent: webcolor!("#e6b450"),
        accent_ink: webcolor!("#765b24"),
        danger: webcolor!("#d95757"),
        editor: webcolor!("#10141c"),
        chip: webcolor!("#141821"),
        warn: webcolor!("#e6b450"),
        keyword: webcolor!("#ff8f40"),
        operator: webcolor!("#f29668"),
        string: webcolor!("#aad94c"),
        number: webcolor!("#d2a6ff"),
        comment: webcolor!("#5a6673"),
    },
    AppTheme {
        id: "ayu-mirage",
        name: "Ayu Mirage",
        dark: true,
        bg: webcolor!("#242936"),
        panel: webcolor!("#1f2430"),
        ink: webcolor!("#fefaecc4"),
        muted: webcolor!("#6e7c8f"),
        line: webcolor!("#171b24"),
        accent: webcolor!("#ffcc66"),
        accent_ink: webcolor!("#735923"),
        danger: webcolor!("#ff6666"),
        editor: webcolor!("#242936"),
        chip: webcolor!("#282e3b"),
        warn: webcolor!("#ffcc66"),
        keyword: webcolor!("#ffa659"),
        operator: webcolor!("#f29e74"),
        string: webcolor!("#d5ff80"),
        number: webcolor!("#dfbfff"),
        comment: webcolor!("#6e7c8f"),
    },
    AppTheme {
        id: "ayu-light",
        name: "Ayu Light",
        dark: false,
        bg: webcolor!("#fcfcfc"),
        panel: webcolor!("#f8f9fa"),
        ink: webcolor!("#020a12a3"),
        muted: webcolor!("#adaeb1"),
        line: webcolor!("#e3e6e8"),
        accent: webcolor!("#f29718"),
        accent_ink: webcolor!("#7e4b01"),
        danger: webcolor!("#e65050"),
        editor: webcolor!("#fcfcfc"),
        chip: webcolor!("#ebeef0"),
        warn: webcolor!("#f29718"),
        keyword: webcolor!("#fa8532"),
        operator: webcolor!("#f2a191"),
        string: webcolor!("#86b300"),
        number: webcolor!("#a37acc"),
        comment: webcolor!("#adaeb1"),
    },
    AppTheme {
        id: "dracula",
        name: "Dracula",
        dark: true,
        bg: webcolor!("#282a36"),
        panel: webcolor!("#21222c"),
        ink: webcolor!("#f8f8f2"),
        muted: webcolor!("#6272a4"),
        line: webcolor!("#44475a"),
        accent: webcolor!("#bd93f9"),
        accent_ink: webcolor!("#282a36"),
        danger: webcolor!("#ff5555"),
        editor: webcolor!("#282a36"),
        chip: webcolor!("#343746"),
        warn: webcolor!("#ffb86c"),
        keyword: webcolor!("#ff79c6"),
        operator: webcolor!("#ff79c6"),
        string: webcolor!("#f1fa8c"),
        number: webcolor!("#bd93f9"),
        comment: webcolor!("#7d8ec4"),
    },
    AppTheme {
        id: "nord",
        name: "Nord",
        dark: true,
        bg: webcolor!("#2e3440"),
        panel: webcolor!("#3b4252"),
        ink: webcolor!("#d8dee9"),
        muted: webcolor!("#7b88a1"),
        line: webcolor!("#4c566a"),
        accent: webcolor!("#88c0d0"),
        accent_ink: webcolor!("#2e3440"),
        danger: webcolor!("#bf616a"),
        editor: webcolor!("#2e3440"),
        chip: webcolor!("#434c5e"),
        warn: webcolor!("#ebcb8b"),
        keyword: webcolor!("#81a1c1"),
        operator: webcolor!("#81a1c1"),
        string: webcolor!("#a3be8c"),
        number: webcolor!("#b590af"),
        comment: webcolor!("#7b88a1"),
    },
    AppTheme {
        id: "solarized-light",
        name: "Solarized Light",
        dark: false,
        bg: webcolor!("#fdf6e3"),
        panel: webcolor!("#eee8d5"),
        ink: webcolor!("#586e75"),
        muted: webcolor!("#657b83"),
        line: webcolor!("#d6d0bc"),
        accent: webcolor!("#268bd2"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#dc322f"),
        editor: webcolor!("#fdf6e3"),
        chip: webcolor!("#eee8d5"),
        warn: webcolor!("#b58900"),
        keyword: webcolor!("#697800"),
        operator: webcolor!("#697800"),
        string: webcolor!("#217e77"),
        number: webcolor!("#ca347d"),
        comment: webcolor!("#657b83"),
    },
    AppTheme {
        id: "solarized-dark",
        name: "Solarized Dark",
        dark: true,
        bg: webcolor!("#002b36"),
        panel: webcolor!("#073642"),
        ink: webcolor!("#93a1a1"),
        muted: webcolor!("#839496"),
        line: webcolor!("#094552"),
        accent: webcolor!("#268bd2"),
        accent_ink: webcolor!("#ffffff"),
        danger: webcolor!("#dc322f"),
        editor: webcolor!("#002b36"),
        chip: webcolor!("#073642"),
        warn: webcolor!("#b58900"),
        keyword: webcolor!("#859900"),
        operator: webcolor!("#859900"),
        string: webcolor!("#2aa198"),
        number: webcolor!("#dd629d"),
        comment: webcolor!("#839496"),
    },
    AppTheme {
        id: "monokai",
        name: "Monokai",
        dark: true,
        bg: webcolor!("#272822"),
        panel: webcolor!("#1e1f1c"),
        ink: webcolor!("#f8f8f2"),
        muted: webcolor!("#90886f"),
        line: webcolor!("#3e3d32"),
        accent: webcolor!("#a6e22e"),
        accent_ink: webcolor!("#272822"),
        danger: webcolor!("#f92672"),
        editor: webcolor!("#272822"),
        chip: webcolor!("#3e3d32"),
        warn: webcolor!("#e6db74"),
        keyword: webcolor!("#fa4988"),
        operator: webcolor!("#fa4988"),
        string: webcolor!("#e6db74"),
        number: webcolor!("#ae81ff"),
        comment: webcolor!("#90886f"),
    },
    AppTheme {
        id: "tokyo-night",
        name: "Tokyo Night",
        dark: true,
        bg: webcolor!("#1a1b26"),
        panel: webcolor!("#16161e"),
        ink: webcolor!("#c0caf5"),
        muted: webcolor!("#737aa2"),
        line: webcolor!("#292e42"),
        accent: webcolor!("#7aa2f7"),
        accent_ink: webcolor!("#1a1b26"),
        danger: webcolor!("#f7768e"),
        editor: webcolor!("#1a1b26"),
        chip: webcolor!("#222433"),
        warn: webcolor!("#e0af68"),
        keyword: webcolor!("#9d7cd8"),
        operator: webcolor!("#89ddff"),
        string: webcolor!("#9ece6a"),
        number: webcolor!("#ff9e64"),
        comment: webcolor!("#737aa2"),
    },
    AppTheme {
        id: "gruvbox-dark",
        name: "Gruvbox Dark",
        dark: true,
        bg: webcolor!("#282828"),
        panel: webcolor!("#282828"),
        ink: webcolor!("#ebdbb2"),
        muted: webcolor!("#a89984"),
        line: webcolor!("#3c3836"),
        accent: webcolor!("#fe8019"),
        accent_ink: webcolor!("#282828"),
        danger: webcolor!("#fb4934"),
        editor: webcolor!("#282828"),
        chip: webcolor!("#3c3836"),
        warn: webcolor!("#d79921"),
        keyword: webcolor!("#fb4934"),
        operator: webcolor!("#fe8019"),
        string: webcolor!("#b8bb26"),
        number: webcolor!("#d3869b"),
        comment: webcolor!("#a89984"),
    },
    AppTheme {
        id: "gruvbox-light",
        name: "Gruvbox Light",
        dark: false,
        bg: webcolor!("#fbf1c7"),
        panel: webcolor!("#fbf1c7"),
        ink: webcolor!("#3c3836"),
        muted: webcolor!("#7c6f64"),
        line: webcolor!("#d5c4a1"),
        accent: webcolor!("#af3a03"),
        accent_ink: webcolor!("#fbf1c7"),
        danger: webcolor!("#9d0006"),
        editor: webcolor!("#fbf1c7"),
        chip: webcolor!("#ebdbb2"),
        warn: webcolor!("#b57614"),
        keyword: webcolor!("#9d0006"),
        operator: webcolor!("#af3a03"),
        string: webcolor!("#79740e"),
        number: webcolor!("#8f3f71"),
        comment: webcolor!("#7c6f64"),
    },
];

pub fn find_theme(id: &str) -> &'static AppTheme {
    THEMES
        .iter()
        .find(|theme| theme.id == id)
        .unwrap_or(&THEMES[0])
}

pub fn choose_readable_color(dark: bool) -> WebColor {
    if dark {
        webcolor!("#f4f4f5")
    } else {
        webcolor!("#1c1c1e")
    }
}

pub fn css_vars(theme: &AppTheme) -> String {
    format!(
        "--bg:{bg};--panel:{panel};--ink:{ink};--muted:{muted};--line:{line};--accent:{accent};--accent-ink:{accent_ink};--danger:{danger};--editor:{editor};--chip:{chip};--warn:{warn};--readable:{readable};--syn-keyword:{keyword};--syn-operator:{operator};--syn-string:{string};--syn-number:{number};--syn-comment:{comment}",
        bg = theme.bg.css(),
        panel = theme.panel.css(),
        ink = theme.ink.css(),
        muted = theme.muted.css(),
        line = theme.line.css(),
        accent = theme.accent.css(),
        accent_ink = theme.accent_ink.css(),
        danger = theme.danger.css(),
        editor = theme.editor.css(),
        chip = theme.chip.css(),
        warn = theme.warn.css(),
        readable = choose_readable_color(theme.dark).css(),
        keyword = theme.keyword.ensure_contrast(theme.editor).css(),
        operator = theme.operator.ensure_contrast(theme.editor).css(),
        string = theme.string.ensure_contrast(theme.editor).css(),
        number = theme.number.ensure_contrast(theme.editor).css(),
        comment = theme.comment.ensure_contrast(theme.editor).css(),
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
        assert_eq!(gruvbox.keyword, "#fb4934");
        assert_eq!(gruvbox.string, "#b8bb26");
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
        for theme in THEMES {
            let style = css_vars(theme);
            let readable = choose_readable_color(theme.dark);
            assert!(
                readable.contrast_with(theme.editor) >= 7.0
                    && readable.contrast_with(theme.panel) >= 7.0,
                "{}",
                theme.id
            );
            assert!(style.contains(&format!("--readable:{}", readable.css())));
            for (name, color) in [
                ("keyword", theme.keyword),
                ("operator", theme.operator),
                ("string", theme.string),
                ("number", theme.number),
                ("comment", theme.comment),
            ] {
                let shown = color.ensure_contrast(theme.editor);
                assert!(
                    shown.contrast_with(theme.editor) >= 4.5,
                    "{} {name}",
                    theme.id
                );
                assert!(style.contains(&format!("--syn-{name}:{}", shown.css())));
            }
        }
        let mut ids = THEMES.iter().map(|theme| theme.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), THEMES.len());
    }
}
