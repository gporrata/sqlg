use sqlparser::dialect::MySqlDialect;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};

use crate::split::location_offset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlTone {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlPiece {
    pub text: String,
    pub tone: SqlTone,
}

pub fn highlight_sql(sql: &str) -> Vec<SqlPiece> {
    let dialect = MySqlDialect {};
    let mut tokenizer = Tokenizer::new(&dialect, sql);
    let Ok(tokens) = tokenizer.tokenize_with_location() else {
        return plain(sql);
    };
    let mut pieces = Vec::new();
    let mut cursor = 0usize;
    for token in tokens {
        if matches!(token.token, Token::EOF) {
            continue;
        }
        let mut start = location_offset(sql, token.span.start.line, token.span.start.column);
        let mut end = location_offset(sql, token.span.end.line, token.span.end.column);
        if start < cursor {
            start = cursor;
        }
        if end < start {
            end = start;
        }
        if end > sql.len() {
            end = sql.len();
        }
        if start > sql.len() {
            break;
        }
        if start > cursor {
            push_piece(&mut pieces, &sql[cursor..start], SqlTone::Plain);
        }
        if end > start {
            push_piece(&mut pieces, &sql[start..end], tone_of(&token.token));
            cursor = end;
        }
    }
    if cursor < sql.len() {
        push_piece(&mut pieces, &sql[cursor..], SqlTone::Plain);
    }
    if pieces.is_empty() {
        return plain(sql);
    }
    pieces
}

fn plain(sql: &str) -> Vec<SqlPiece> {
    if sql.is_empty() {
        Vec::new()
    } else {
        vec![SqlPiece {
            text: sql.to_string(),
            tone: SqlTone::Plain,
        }]
    }
}

fn push_piece(pieces: &mut Vec<SqlPiece>, text: &str, tone: SqlTone) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = pieces.last_mut()
        && last.tone == tone
    {
        last.text.push_str(text);
        return;
    }
    pieces.push(SqlPiece {
        text: text.to_string(),
        tone,
    });
}

fn tone_of(token: &Token) -> SqlTone {
    match token {
        Token::Word(word) if word.quote_style.is_none() && word.keyword != Keyword::NoKeyword => {
            SqlTone::Keyword
        }
        Token::Number(_, _) => SqlTone::Number,
        Token::Whitespace(
            Whitespace::SingleLineComment { .. } | Whitespace::MultiLineComment(_),
        ) => SqlTone::Comment,
        token if string_token(token) => SqlTone::String,
        token if operator_token(token) => SqlTone::Operator,
        _ => SqlTone::Plain,
    }
}

fn string_token(token: &Token) -> bool {
    matches!(
        token,
        Token::SingleQuotedString(_)
            | Token::DoubleQuotedString(_)
            | Token::TripleSingleQuotedString(_)
            | Token::TripleDoubleQuotedString(_)
            | Token::DollarQuotedString(_)
            | Token::NationalStringLiteral(_)
            | Token::EscapedStringLiteral(_)
            | Token::UnicodeStringLiteral(_)
            | Token::HexStringLiteral(_)
            | Token::SingleQuotedByteStringLiteral(_)
            | Token::DoubleQuotedByteStringLiteral(_)
            | Token::SingleQuotedRawStringLiteral(_)
            | Token::DoubleQuotedRawStringLiteral(_)
    )
}

fn operator_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Eq
            | Token::Neq
            | Token::Lt
            | Token::Gt
            | Token::LtEq
            | Token::GtEq
            | Token::DoubleEq
            | Token::Spaceship
            | Token::Plus
            | Token::Minus
            | Token::Mul
            | Token::Div
            | Token::Mod
            | Token::StringConcat
            | Token::Placeholder(_)
            | Token::CustomBinaryOperator(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(sql: &str) -> String {
        highlight_sql(sql)
            .into_iter()
            .map(|piece| piece.text)
            .collect()
    }

    fn tone_at(sql: &str, needle: &str) -> Option<SqlTone> {
        highlight_sql(sql)
            .into_iter()
            .find(|piece| piece.text == needle)
            .map(|piece| piece.tone)
    }

    #[test]
    fn highlight_covers_the_script_exactly() {
        let sql = "select id, name\nfrom trips\nwhere cost >= 10 and note = 'a''b' -- pending\n# hash\n/* block\ncomment */\n";
        assert_eq!(joined(sql), sql);
        assert_eq!(tone_at(sql, "select"), Some(SqlTone::Keyword));
        assert_eq!(tone_at(sql, "from"), Some(SqlTone::Keyword));
        assert_eq!(tone_at(sql, "where"), Some(SqlTone::Keyword));
        assert_eq!(tone_at(sql, "10"), Some(SqlTone::Number));
        assert_eq!(tone_at(sql, "'a''b'"), Some(SqlTone::String));
        assert_eq!(tone_at(sql, ">="), Some(SqlTone::Operator));
        assert!(
            highlight_sql(sql)
                .iter()
                .any(|piece| piece.tone == SqlTone::Comment && piece.text.contains("pending"))
        );
        assert!(
            highlight_sql(sql)
                .iter()
                .any(|piece| piece.tone == SqlTone::Comment && piece.text.starts_with('#'))
        );
        let quoted = highlight_sql("select `select`");
        assert_eq!(joined("select `select`"), "select `select`");
        assert!(
            quoted
                .iter()
                .any(|piece| { piece.tone == SqlTone::Keyword && piece.text == "select" })
        );
        assert!(
            quoted
                .iter()
                .any(|piece| { piece.tone == SqlTone::Plain && piece.text.contains("`select`") })
        );
        assert_eq!(joined(""), "");
        assert_eq!(joined("not sql !!!"), "not sql !!!");
    }
}
