use sqlparser::dialect::MySqlDialect;
use sqlparser::tokenizer::{Token, Tokenizer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlStatement {
    pub sql: String,
    pub start: usize,
    pub end: usize,
}

/// Split a script into statements. `DELIMITER` client commands are not
/// supported: the whole script is returned as one statement.
pub fn split_statements(input: &str) -> Vec<SqlStatement> {
    if input.trim().is_empty() {
        return Vec::new();
    }
    if uses_delimiter_command(input) {
        return whole(input);
    }
    match split_tokens(input) {
        Ok(parts) if !parts.is_empty() => parts,
        _ => split_scanned(input),
    }
}

pub fn statement_at(input: &str, cursor: usize) -> Option<SqlStatement> {
    let parts = split_statements(input);
    parts
        .iter()
        .find(|part| cursor >= part.start && cursor <= part.end)
        .cloned()
        .or_else(|| parts.into_iter().rev().find(|part| part.start <= cursor))
}

pub fn snippet(sql: &str, max_chars: usize) -> String {
    let collapsed = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    for ch in collapsed.chars() {
        if out.chars().count() >= max_chars {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

fn whole(input: &str) -> Vec<SqlStatement> {
    let trimmed = input.trim();
    let start = input.find(trimmed).unwrap_or(0);
    vec![SqlStatement {
        sql: trimmed.to_string(),
        start,
        end: start + trimmed.len(),
    }]
}

fn uses_delimiter_command(input: &str) -> bool {
    input.lines().any(|line| {
        let trimmed = line.trim();
        let rest = trimmed
            .get(.."delimiter".len())
            .filter(|head| head.eq_ignore_ascii_case("delimiter"));
        match rest {
            Some(_) if trimmed.len() == "delimiter".len() => true,
            Some(_) => trimmed.as_bytes()["delimiter".len()].is_ascii_whitespace(),
            None => false,
        }
    })
}

fn split_tokens(input: &str) -> Result<Vec<SqlStatement>, ()> {
    let dialect = MySqlDialect {};
    let mut tokenizer = Tokenizer::new(&dialect, input);
    let tokens = tokenizer.tokenize_with_location().map_err(|_| ())?;
    let mut parts = Vec::new();
    let mut start = 0usize;
    for token in &tokens {
        if matches!(token.token, Token::SemiColon) {
            let end = span_end(
                input,
                token.span.start.line,
                token.span.start.column,
                token.span.end.line,
                token.span.end.column,
            );
            push_part(&mut parts, input, start, end);
            start = end;
        }
    }
    push_part(&mut parts, input, start, input.len());
    Ok(parts)
}

fn span_end(input: &str, start_line: u64, start_col: u64, end_line: u64, end_col: u64) -> usize {
    let _ = (start_line, start_col);
    // Token spans from sqlparser are exclusive at the end location.
    offset_at(input, end_line, end_col).min(input.len())
}

fn offset_at(input: &str, line: u64, column: u64) -> usize {
    if line == 0 {
        return 0;
    }
    let mut current_line = 1u64;
    let mut idx = 0usize;
    if current_line == line {
        return advance_columns(input, idx, column);
    }
    for (i, ch) in input.char_indices() {
        if ch == '\n' {
            current_line += 1;
            idx = i + ch.len_utf8();
            if current_line == line {
                return advance_columns(input, idx, column);
            }
        }
    }
    input.len()
}

fn advance_columns(input: &str, base: usize, column: u64) -> usize {
    let mut col = column.saturating_sub(1) as usize;
    let mut idx = base;
    for ch in input[base..].chars() {
        if col == 0 {
            break;
        }
        idx += ch.len_utf8();
        col -= 1;
    }
    idx.min(input.len())
}

fn push_part(out: &mut Vec<SqlStatement>, input: &str, start: usize, end: usize) {
    if start >= end || end > input.len() {
        return;
    }
    let slice = &input[start..end];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    if lead + trail >= slice.len() {
        return;
    }
    let from = start + lead;
    let to = end - trail;
    out.push(SqlStatement {
        sql: input[from..to].to_string(),
        start: from,
        end: to,
    });
}

fn split_scanned(input: &str) -> Vec<SqlStatement> {
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    #[derive(Clone, Copy)]
    enum Mode {
        Normal,
        SQuote,
        DQuote,
        BQuote,
        Line,
        Block,
    }
    let mut mode = Mode::Normal;
    while i < chars.len() {
        let (idx, ch) = chars[i];
        let next = chars.get(i + 1).map(|(_, c)| *c);
        match mode {
            Mode::Normal => match ch {
                '\'' => mode = Mode::SQuote,
                '"' => mode = Mode::DQuote,
                '`' => mode = Mode::BQuote,
                '#' => mode = Mode::Line,
                '-' if next == Some('-') && is_mysql_line_comment(&chars, i) => {
                    mode = Mode::Line;
                    i += 1;
                }
                '/' if next == Some('*') => {
                    mode = Mode::Block;
                    i += 1;
                }
                ';' => {
                    push_part(&mut parts, input, start, idx + ch.len_utf8());
                    start = idx + ch.len_utf8();
                }
                _ => {}
            },
            Mode::SQuote => {
                if ch == '\\' {
                    i += 1;
                } else if ch == '\'' {
                    if next == Some('\'') {
                        i += 1;
                    } else {
                        mode = Mode::Normal;
                    }
                }
            }
            Mode::DQuote => {
                if ch == '\\' {
                    i += 1;
                } else if ch == '"' {
                    if next == Some('"') {
                        i += 1;
                    } else {
                        mode = Mode::Normal;
                    }
                }
            }
            Mode::BQuote => {
                if ch == '`' {
                    if next == Some('`') {
                        i += 1;
                    } else {
                        mode = Mode::Normal;
                    }
                }
            }
            Mode::Line => {
                if ch == '\n' {
                    mode = Mode::Normal;
                }
            }
            Mode::Block => {
                if ch == '*' && next == Some('/') {
                    mode = Mode::Normal;
                    i += 1;
                }
            }
        }
        i += 1;
    }
    push_part(&mut parts, input, start, input.len());
    parts
}

fn is_mysql_line_comment(chars: &[(usize, char)], dash_at: usize) -> bool {
    match chars.get(dash_at + 2).map(|(_, c)| *c) {
        None => true,
        Some(ch) => ch.is_whitespace(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quotes_comments_and_keeps_offsets() {
        let sql = "select ';'; select 1; -- trailing;\nselect 2";
        let parts = split_statements(sql);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].sql, "select ';';");
        assert_eq!(parts[1].sql, "select 1;");
        assert!(parts[2].sql.contains("select 2"));
        assert_eq!(&sql[parts[0].start..parts[0].end], parts[0].sql);
    }

    #[test]
    fn delimiter_is_one_statement() {
        let sql = "DELIMITER //\nSELECT 1//\nDELIMITER ;";
        let parts = split_statements(sql);
        assert_eq!(parts.len(), 1);
        assert!(parts[0].sql.contains("DELIMITER"));
    }

    #[test]
    fn snippet_collapses_whitespace() {
        assert_eq!(snippet("select\n  1", 20), "select 1");
        assert!(snippet(&"a".repeat(80), 10).ends_with('…'));
    }
}
