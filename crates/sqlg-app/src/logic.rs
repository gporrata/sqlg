use sqlg_core::{LintCatalog, LintSchema, LintTable};
use sqlg_mysql::SchemaSnapshot;

pub const VISIBLE_ROWS: usize = 100;
pub const COLUMN_MIN: f64 = 72.0;
pub const SCHEMA_MIN: f32 = 160.0;
pub const SCHEMA_MAX: f32 = 600.0;
pub const EDITOR_MIN: f32 = 220.0;
pub const EDITOR_MAX: f32 = 1200.0;
pub const EDITOR_HEIGHT_MIN: f32 = 140.0;
pub const EDITOR_HEIGHT_MAX: f32 = 800.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panes {
    pub schema: bool,
    pub editor: bool,
    pub results: bool,
    pub stack: bool,
}

pub fn layout_panes(layout: &str) -> Panes {
    Panes {
        schema: layout == "all" || layout == "below",
        editor: layout != "results",
        results: layout != "editor",
        stack: layout == "below" || layout == "below-only",
    }
}

pub fn known_layout(layout: &str) -> bool {
    matches!(
        layout,
        "all" | "editor-results" | "below" | "below-only" | "editor" | "results"
    )
}

pub fn clamp_pane(size: f32, min: f32, max: f32) -> f32 {
    let upper = min.max(max);
    size.clamp(min, upper)
}

pub fn pane_size(value: Option<f64>, fallback: f32, min: f32) -> f32 {
    let Some(value) = value else {
        return fallback;
    };
    if !value.is_finite() {
        return fallback;
    }
    (value as f32).clamp(min, 4000.0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedTitle {
    pub label: String,
    pub tooltip: String,
    pub placeholder: bool,
}

pub fn derive_title(sql: &str) -> DerivedTitle {
    if let Some(comment) = first_comment_text(sql) {
        let full = collapse(&comment);
        if !full.is_empty() {
            return DerivedTitle {
                label: truncate(&full, 30),
                tooltip: full,
                placeholder: false,
            };
        }
    }
    let query = collapse(&first_statement(sql));
    if query.is_empty() {
        return DerivedTitle {
            label: String::new(),
            tooltip: String::new(),
            placeholder: true,
        };
    }
    DerivedTitle {
        label: truncate(&query, 30),
        tooltip: query,
        placeholder: false,
    }
}

pub fn tab_label(sql: &str) -> String {
    let title = derive_title(sql);
    if title.placeholder {
        "New query".into()
    } else {
        title.label
    }
}

pub fn tab_tooltip(sql: &str, file_path: Option<&str>) -> String {
    let title = derive_title(sql);
    let base = if title.placeholder {
        "New query".to_string()
    } else {
        title.tooltip
    };
    match file_path {
        Some(path) => format!("{base} — {path}"),
        None => base,
    }
}

pub fn truncate(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars).collect();
    out.push('…');
    out
}

pub fn collapse(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn execution_summary(total: usize, executed: usize, failed: usize) -> String {
    if total == 0 {
        return "Nothing to run".into();
    }
    if failed == 0 {
        return if total == 1 {
            "1 statement executed".into()
        } else {
            format!("{total} statements executed")
        };
    }
    let fail = if failed == 1 {
        "1 failed".to_string()
    } else {
        format!("{failed} failed")
    };
    format!("{executed} of {total} statements executed · {fail}")
}

pub fn format_elapsed(ms: Option<u64>) -> String {
    let Some(ms) = ms else {
        return String::new();
    };
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.2} s", ms as f64 / 1000.0)
    }
}

pub fn is_dirty(sql: &str, file_path: Option<&str>, saved_sql: Option<&str>) -> bool {
    if file_path.is_some() {
        sql != saved_sql.unwrap_or("")
    } else {
        !sql.is_empty()
    }
}

pub fn needs_file_save(sql: &str, file_path: Option<&str>, saved_sql: Option<&str>) -> bool {
    file_path.is_some() && sql != saved_sql.unwrap_or("")
}

pub fn badge_color(name: &str, color: Option<&str>) -> String {
    if let Some(raw) = color.map(str::trim).filter(|value| !value.is_empty()) {
        return parse_color(raw);
    }
    let lower = name.to_ascii_lowercase();
    if lower.contains("prod") {
        return named_color("red");
    }
    if lower.contains("dev") {
        return named_color("green");
    }
    if lower.contains("preview") {
        return named_color("blue");
    }
    if lower.contains("test") {
        return named_color("orange");
    }
    let mut hash: u32 = 0;
    for ch in name.chars() {
        hash = hash.wrapping_mul(31).wrapping_add(ch as u32);
    }
    const PALETTE: [&str; 6] = ["green", "red", "blue", "orange", "purple", "teal"];
    named_color(PALETTE[(hash as usize) % PALETTE.len()])
}

fn named_color(name: &str) -> String {
    match name {
        "red" => "#c0392b",
        "green" => "#1e8449",
        "blue" => "#2471a3",
        "orange" => "#d68910",
        "purple" => "#7d3c98",
        "teal" => "#148f77",
        _ => "#566573",
    }
    .into()
}

fn parse_color(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "red" | "green" | "blue" | "orange" | "purple" | "teal"
    ) {
        return named_color(&lower);
    }
    let hex = lower.trim_start_matches('#');
    if hex.len() == 6 && u32::from_str_radix(hex, 16).is_ok() {
        return format!("#{hex}");
    }
    named_color("teal")
}

pub fn utf16_to_byte(text: &str, utf16_index: usize) -> usize {
    let mut units = 0usize;
    for (byte, ch) in text.char_indices() {
        if units >= utf16_index {
            return byte;
        }
        units += ch.len_utf16();
    }
    text.len()
}

pub fn byte_to_utf16(text: &str, byte: usize) -> u32 {
    let mut units = 0u32;
    for (index, ch) in text.char_indices() {
        if index >= byte {
            break;
        }
        units += ch.len_utf16() as u32;
    }
    units
}

pub fn quote_ident(name: &str) -> String {
    let safe = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !name.chars().next().unwrap().is_ascii_digit();
    if safe {
        name.to_string()
    } else {
        format!("`{}`", name.replace('`', "``"))
    }
}

pub fn offset_to_position(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut column = 0usize;
    let mut seen = 0usize;
    for ch in text.chars() {
        if seen >= offset {
            break;
        }
        let width = ch.len_utf8();
        if seen + width > offset {
            break;
        }
        seen += width;
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

pub fn position_to_offset(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0usize;
    let mut current_line = 0usize;
    let mut current_column = 0usize;
    for ch in text.chars() {
        if current_line == line && current_column == column {
            return offset;
        }
        offset += ch.len_utf8();
        if ch == '\n' {
            if current_line == line {
                return offset - 1;
            }
            current_line += 1;
            current_column = 0;
        } else {
            current_column += 1;
        }
    }
    offset
}

pub fn splice_text(sql: &str, from: usize, to: usize, insert: &str) -> Option<String> {
    if from > to || to > sql.len() || !sql.is_char_boundary(from) || !sql.is_char_boundary(to) {
        return None;
    }
    let mut next = String::with_capacity(sql.len() - (to - from) + insert.len());
    next.push_str(&sql[..from]);
    next.push_str(insert);
    next.push_str(&sql[to..]);
    Some(next)
}

pub fn catalog_for(
    snapshot: Option<&SchemaSnapshot>,
    default_schema: Option<String>,
) -> LintCatalog {
    let Some(snapshot) = snapshot else {
        return LintCatalog {
            default_schema,
            ..LintCatalog::default()
        };
    };
    LintCatalog {
        default_schema,
        lower_case_table_names: snapshot.lower_case_table_names,
        schemas: snapshot
            .schemas
            .iter()
            .map(|schema| LintSchema {
                name: schema.name.clone(),
                tables: schema
                    .tables
                    .iter()
                    .map(|table| LintTable {
                        name: table.name.clone(),
                        columns: table
                            .columns
                            .iter()
                            .map(|column| column.name.clone())
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn first_comment_text(sql: &str) -> Option<String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if ch == '\'' || ch == '"' || ch == '`' {
            index = skip_quote(&chars, index, ch);
            continue;
        }
        if ch == '#' {
            return Some(line_rest(&chars, index + 1));
        }
        if ch == '-' && next == Some('-') && is_line_comment(&chars, index + 2) {
            return Some(line_rest(&chars, index + 2));
        }
        if ch == '/' && next == Some('*') {
            if chars.get(index + 2) == Some(&'!') {
                index = find_block_end(&chars, index + 3);
                continue;
            }
            let end = chars[index + 2..]
                .windows(2)
                .position(|pair| pair == ['*', '/'])
                .map(|pos| index + 2 + pos);
            let body: String = match end {
                Some(end) => chars[index + 2..end].iter().collect(),
                None => chars[index + 2..].iter().collect(),
            };
            let line = body
                .split('\n')
                .map(str::trim)
                .find(|part| !part.is_empty())
                .unwrap_or("");
            return Some(line.to_string());
        }
        index += 1;
    }
    None
}

fn is_line_comment(chars: &[char], at: usize) -> bool {
    chars.get(at).is_none_or(|ch| ch.is_whitespace())
}

fn line_rest(chars: &[char], at: usize) -> String {
    let end = chars[at..]
        .iter()
        .position(|ch| *ch == '\n')
        .map(|pos| at + pos)
        .unwrap_or(chars.len());
    chars[at..end].iter().collect()
}

fn skip_quote(chars: &[char], start: usize, quote: char) -> usize {
    let mut index = start + 1;
    while index < chars.len() {
        if chars[index] == '\\' {
            index += 2;
            continue;
        }
        if chars[index] == quote {
            if chars.get(index + 1) == Some(&quote) {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += 1;
    }
    chars.len()
}

fn find_block_end(chars: &[char], at: usize) -> usize {
    chars[at..]
        .windows(2)
        .position(|pair| pair == ['*', '/'])
        .map(|pos| at + pos + 2)
        .unwrap_or(chars.len())
}

fn first_statement(sql: &str) -> String {
    let chars: Vec<char> = sql.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if ch == '\'' || ch == '"' || ch == '`' {
            index = skip_quote(&chars, index, ch);
            continue;
        }
        if ch == '#' || (ch == '-' && next == Some('-') && is_line_comment(&chars, index + 2)) {
            index = chars[index..]
                .iter()
                .position(|item| *item == '\n')
                .map(|pos| index + pos + 1)
                .unwrap_or(chars.len());
            continue;
        }
        if ch == '/' && next == Some('*') {
            index = find_block_end(&chars, index + 2);
            continue;
        }
        if ch == ';' {
            return chars[..index].iter().collect();
        }
        index += 1;
    }
    sql.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnMark {
    PrimaryKey,
    ForeignKey,
    Numeric,
    Text,
    Datetime,
    Enum,
    Other,
}

pub fn classify_column(
    column_key: &str,
    foreign_key: bool,
    column_type: &str,
    data_type: &str,
) -> ColumnMark {
    if column_key.eq_ignore_ascii_case("PRI") {
        return ColumnMark::PrimaryKey;
    }
    if foreign_key {
        return ColumnMark::ForeignKey;
    }
    match column_tone(column_type, data_type) {
        "enum" => ColumnMark::Enum,
        "time" => ColumnMark::Datetime,
        "num" => ColumnMark::Numeric,
        "text" => ColumnMark::Text,
        _ => ColumnMark::Other,
    }
}

pub fn column_tone(column_type: &str, data_type: &str) -> &'static str {
    let tone = type_tone(if column_type.is_empty() {
        data_type
    } else {
        column_type
    });
    if tone == "other" && !data_type.is_empty() {
        type_tone(data_type)
    } else {
        tone
    }
}

fn type_tone(type_name: &str) -> &'static str {
    let data = type_name.to_ascii_lowercase();
    if data == "bool"
        || data == "boolean"
        || data == "enum"
        || data == "set"
        || data.starts_with("enum(")
        || data.starts_with("set(")
        || data.starts_with("tinyint(1)")
    {
        return "enum";
    }
    if data == "year" || data.contains("date") || data.contains("time") {
        return "time";
    }
    if [
        "int", "decimal", "numeric", "float", "double", "bit", "real", "serial", "dec",
    ]
    .iter()
    .any(|needle| data.contains(needle))
    {
        return "num";
    }
    if ["char", "text", "blob", "binary", "json"]
        .iter()
        .any(|needle| data.contains(needle))
    {
        return "text";
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_uses_the_first_comment_and_skips_strings() {
        let title = derive_title("select '-- not a title';\n-- Monthly report\nselect 1");
        assert_eq!(title.label, "Monthly report");
        assert!(!title.placeholder);
        let version = derive_title("/*! keep */\n# trips\nselect 1");
        assert_eq!(version.tooltip, "trips");
        let empty = derive_title("   \n");
        assert!(empty.placeholder);
        assert_eq!(tab_label(""), "New query");
    }

    #[test]
    fn block_comment_uses_the_first_line() {
        let title = derive_title("/* line one\nline two */\nselect 1");
        assert_eq!(title.tooltip, "line one");
        assert_eq!(
            truncate("abcdefghijklmnopqrstuvwxyz0123456789", 30)
                .chars()
                .count(),
            31
        );
    }

    #[test]
    fn dirty_tracks_linked_files_only_for_prompts() {
        assert!(is_dirty("select 1", None, None));
        assert!(!is_dirty("", None, None));
        assert!(!needs_file_save("select 1", None, None));
        assert!(needs_file_save("select 2", Some("a.sql"), Some("select 1")));
        assert!(!needs_file_save(
            "select 1",
            Some("a.sql"),
            Some("select 1")
        ));
    }

    #[test]
    fn summary_and_elapsed_match_the_footer() {
        assert_eq!(execution_summary(0, 0, 0), "Nothing to run");
        assert_eq!(execution_summary(1, 1, 0), "1 statement executed");
        assert_eq!(
            execution_summary(3, 2, 1),
            "2 of 3 statements executed · 1 failed"
        );
        assert_eq!(format_elapsed(Some(12)), "12 ms");
        assert_eq!(format_elapsed(Some(1500)), "1.50 s");
    }

    #[test]
    fn layouts_and_badges() {
        assert!(layout_panes("all").schema);
        assert!(layout_panes("below").stack);
        assert!(!layout_panes("editor").results);
        assert!(!layout_panes("results").editor);
        assert!(!layout_panes("editor-results").schema);
        assert_eq!(clamp_pane(10.0, 160.0, 600.0), 160.0);
        assert_eq!(badge_color("prod-main", None), "#c0392b");
        assert_eq!(badge_color("local", Some("#112233")), "#112233");
        assert_eq!(utf16_to_byte("cé", 1), 1);
        assert_eq!(utf16_to_byte("cé", 2), 3);
        assert_eq!(byte_to_utf16("cé", 3), 2);
    }

    #[test]
    fn offsets_round_trip_on_newlines() {
        let sql = "ab\ncé";
        assert_eq!(offset_to_position(sql, 0), (0, 0));
        assert_eq!(offset_to_position(sql, 3), (1, 0));
        assert_eq!(position_to_offset(sql, 1, 1), 4);
        assert_eq!(
            splice_text("select *", 7, 8, "id"),
            Some("select id".into())
        );
        assert_eq!(quote_ident("trips"), "trips");
        assert_eq!(quote_ident("a b"), "`a b`");
    }

    #[test]
    fn column_marks_follow_key_then_type() {
        assert_eq!(
            classify_column("PRI", true, "int", "int"),
            ColumnMark::PrimaryKey
        );
        assert_eq!(
            classify_column("MUL", true, "bigint", "bigint"),
            ColumnMark::ForeignKey
        );
        assert_eq!(
            classify_column("", false, "varchar(32)", "varchar"),
            ColumnMark::Text
        );
        assert_eq!(
            classify_column("", false, "datetime", "datetime"),
            ColumnMark::Datetime
        );
        assert_eq!(
            classify_column("", false, "decimal(10,2)", "decimal"),
            ColumnMark::Numeric
        );
        assert_eq!(
            classify_column("", false, "enum('a','b')", "enum"),
            ColumnMark::Enum
        );
        assert_eq!(
            classify_column("", false, "tinyint(1)", "tinyint"),
            ColumnMark::Enum
        );
        assert_eq!(classify_column("", false, "", "json"), ColumnMark::Text);
        assert_eq!(
            classify_column("", false, "geometry", "geometry"),
            ColumnMark::Other
        );
        assert_eq!(column_tone("int unsigned", "int"), "num");
        assert_eq!(column_tone("", "text"), "text");
    }
}
