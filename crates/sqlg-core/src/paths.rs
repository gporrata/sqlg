use std::path::PathBuf;

use crate::BUNDLE_ID;

/// `~/Library/Application Support/<bundle-id>`, or `SQLG_DATA_DIR` when set.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SQLG_DATA_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(BUNDLE_ID)
}
