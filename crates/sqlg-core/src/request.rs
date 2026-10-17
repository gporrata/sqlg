use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliRequest {
    pub conn: String,
    pub sql: String,
    /// `tab` or `new-window`.
    pub target: String,
    #[serde(default)]
    pub title: Option<String>,
    pub run: bool,
    pub ack: String,
}

/// Insert `-- <title>` as the first line so the tab-title rule picks it up.
pub fn insert_title(sql: &str, title: &str) -> String {
    let flat: String = title
        .chars()
        .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect();
    format!("-- {flat}\n{sql}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_a_leading_comment_and_newlines_become_spaces() {
        let sql = insert_title("select 1", "Monthly\nreport");
        assert_eq!(sql, "-- Monthly report\nselect 1");
    }

    #[test]
    fn request_round_trip() {
        let req = CliRequest {
            conn: "dev".into(),
            sql: "select 1".into(),
            target: "tab".into(),
            title: Some("n".into()),
            run: true,
            ack: "/tmp/ack".into(),
        };
        let bytes = serde_json::to_vec(&req).unwrap();
        let back: CliRequest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }
}
