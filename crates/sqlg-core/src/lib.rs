//! Shared types for the sqlg app and the `sqlg` CLI.
//! Passwords never live in this crate's on-disk config. Callers put them in
//! the macOS Keychain (or a test double).

mod lint;
mod paths;
mod request;
mod secrets;
mod session;
mod split;
mod store;

pub const BUNDLE_ID: &str = "dev.sqlg.app";

pub use lint::{Diagnostic, LintAction, LintCatalog, LintSchema, LintTable, lint_sql};
pub use paths::data_dir;
pub use request::{CliRequest, insert_title};
pub use secrets::{SecretStore, Secrets};
pub use session::{SessionDocument, TabSnapshot, WindowSnapshot};
pub use split::{SqlStatement, snippet, split_statements, statement_at};
pub use store::{Connection, ConnectionPatch, Store, StoreError, atomic_write};
