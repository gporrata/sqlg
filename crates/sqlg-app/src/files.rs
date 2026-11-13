use std::path::{Path, PathBuf};
use std::process::Command;

use sqlg_core::atomic_write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    pub text: String,
    pub newline: String,
    pub bom: bool,
}

pub fn read_sql_file(path: &Path) -> Result<TextFile, String> {
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let bom = bytes.starts_with(&[0xEF, 0xBB, 0xBF]);
    let body = if bom { &bytes[3..] } else { &bytes };
    let text = String::from_utf8(body.to_vec()).map_err(|_| "file is not UTF-8".to_string())?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    Ok(TextFile {
        text: text.replace("\r\n", "\n"),
        newline: newline.into(),
        bom,
    })
}

pub fn write_sql_file(path: &Path, text: &str, newline: &str, bom: bool) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let ending = if newline == "\r\n" { "\r\n" } else { "\n" };
    let mut body = text.replace("\r\n", "\n").replace('\n', ending);
    if bom {
        body.insert(0, '\u{feff}');
    }
    atomic_write(path, body.as_bytes()).map_err(|err| err.to_string())
}

pub fn file_modified(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

pub fn install_cli() -> Result<String, String> {
    let source = cli_binary()
        .ok_or_else(|| "sqlg command binary was not found beside the app".to_string())?;
    let system = PathBuf::from("/usr/local/bin/sqlg");
    match link_cli(&source, &system) {
        Ok(()) => Ok(format!("linked {}", system.display())),
        Err(err) if system.symlink_metadata().is_ok() => Err(err),
        Err(err) => {
            let local = local_bin();
            if let Some(parent) = local.parent() {
                std::fs::create_dir_all(parent).map_err(|io| io.to_string())?;
            }
            link_cli(&source, &local).map_err(|_| err)?;
            Ok(format!(
                "linked {}. Add {} to PATH if it is not already.",
                local.display(),
                local
                    .parent()
                    .unwrap_or(Path::new("~/.local/bin"))
                    .display()
            ))
        }
    }
}

pub fn uninstall_cli() -> Result<String, String> {
    let mut removed = Vec::new();
    for path in [PathBuf::from("/usr/local/bin/sqlg"), local_bin()] {
        if path.symlink_metadata().is_ok() {
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(path.display().to_string()),
                Err(_) => {
                    let script = format!(
                        "do shell script \"rm -f '{}'\" with administrator privileges",
                        escape_osascript(&path.display().to_string())
                    );
                    let status = Command::new("osascript")
                        .arg("-e")
                        .arg(script)
                        .status()
                        .map_err(|err| err.to_string())?;
                    if status.success() {
                        removed.push(path.display().to_string());
                    }
                }
            }
        }
    }
    if removed.is_empty() {
        Ok("sqlg command was not installed".into())
    } else {
        Ok(format!("removed {}", removed.join(", ")))
    }
}

fn local_bin() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local/bin/sqlg")
}

fn cli_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SQLG_CLI") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let beside = exe.parent()?.join("sqlg");
    if beside.is_file() {
        return Some(beside);
    }
    None
}

fn link_cli(source: &Path, dest: &Path) -> Result<(), String> {
    if dest.symlink_metadata().is_ok()
        && let Err(err) = std::fs::remove_file(dest)
    {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            return admin_replace_link(source, dest);
        }
        return Err(err.to_string());
    }
    match std::os::unix::fs::symlink(source, dest) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            admin_replace_link(source, dest)
        }
        Err(err) => Err(err.to_string()),
    }
}

fn admin_replace_link(source: &Path, dest: &Path) -> Result<(), String> {
    let script = format!(
        "do shell script \"rm -f '{}' && ln -sfn '{}' '{}'\" with administrator privileges",
        escape_osascript(&dest.display().to_string()),
        escape_osascript(&source.display().to_string()),
        escape_osascript(&dest.display().to_string())
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("administrator install was declined".into())
    }
}

fn escape_osascript(value: &str) -> String {
    value.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_bom_and_crlf() {
        let dir = tempfile_dir();
        let path = dir.join("q.sql");
        write_sql_file(&path, "select 1\nselect 2", "\r\n", true).unwrap();
        let file = read_sql_file(&path).unwrap();
        assert_eq!(file.text, "select 1\nselect 2");
        assert_eq!(file.newline, "\r\n");
        assert!(file.bom);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn tempfile_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("sqlg-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
