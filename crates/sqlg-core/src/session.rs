use serde::{Deserialize, Serialize};

fn default_version() -> u32 {
    1
}

fn default_theme() -> String {
    "light".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionDocument {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub front_window: Option<String>,
    #[serde(default)]
    pub windows: Vec<WindowSnapshot>,
}

impl Default for SessionDocument {
    fn default() -> Self {
        Self {
            version: 1,
            theme: "light".into(),
            front_window: None,
            windows: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub active_tab: String,
    #[serde(default)]
    pub tabs: Vec<TabSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TabSnapshot {
    pub id: String,
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default)]
    pub sql: String,
    #[serde(default)]
    pub cursor: u32,
    #[serde(default)]
    pub selection_anchor: Option<u32>,
    #[serde(default)]
    pub selection_head: Option<u32>,
    #[serde(default)]
    pub scroll_top: f64,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub saved_sql: Option<String>,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub newline: String,
}

fn default_layout() -> String {
    "all".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_extra_fields_round_trip() {
        let raw = r#"{
            "version": 1,
            "future": true,
            "windows": [{
                "label": "main",
                "x": 10,
                "y": 20,
                "width": 800,
                "height": 600,
                "tabs": [{"id": "t1", "sql": "select 1", "dirty": true}]
            }]
        }"#;
        let doc: SessionDocument = serde_json::from_str(raw).unwrap();
        assert_eq!(doc.theme, "light");
        assert_eq!(doc.windows[0].tabs[0].sql, "select 1");
        assert!(doc.windows[0].tabs[0].dirty);
        let encoded = serde_json::to_string(&doc).unwrap();
        assert!(!encoded.to_lowercase().contains("password"));
        let again: SessionDocument = serde_json::from_str(&encoded).unwrap();
        assert_eq!(again.windows[0].label, "main");
    }

    #[test]
    fn version_defaults_when_absent() {
        let doc: SessionDocument = serde_json::from_str("{}").unwrap();
        assert_eq!(doc.version, 1);
        assert!(doc.windows.is_empty());
    }
}
