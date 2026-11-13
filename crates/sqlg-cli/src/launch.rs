use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub fn deliver_request(dir: &Path, request_path: &Path) -> bool {
    let socket = dir.join("gui.sock");
    if let Ok(payload) = std::fs::read(request_path)
        && send_payload(&socket, &payload).is_ok()
    {
        return true;
    }
    let Some(app) = app_binary() else {
        return false;
    };
    std::process::Command::new(app)
        .arg("--cli-request")
        .arg(request_path)
        .stdin(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

fn app_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SQLG_APP") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join("sqlg-app");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    if let Some(path) = command_on_path("sqlg-app") {
        return Some(path);
    }
    let bundled = PathBuf::from("/Applications/sqlg.app/Contents/MacOS/sqlg-app");
    if bundled.is_file() {
        return Some(bundled);
    }
    None
}

fn send_payload(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    let started = Instant::now();
    loop {
        match send_once(path, payload) {
            Ok(()) => return Ok(()),
            Err(err) if started.elapsed() >= Duration::from_millis(400) => return Err(err),
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn send_once(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    let mut stream = std::os::unix::net::UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(payload)?;
    if !payload.ends_with(b"\n") {
        stream.write_all(b"\n")?;
    }
    stream.flush()?;
    let mut response = [0u8; 1];
    let _ = stream.read(&mut response);
    Ok(())
}

fn command_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
