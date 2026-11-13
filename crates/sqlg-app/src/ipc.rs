use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use fs4::fs_std::FileExt;
use serde_json::Value;
use sqlg_core::CliRequest;

pub struct Primary {
    pub incoming: Receiver<Incoming>,
    pub startup: Vec<CliRequest>,
    pub _lock: std::fs::File,
}

#[derive(Debug, Clone)]
pub enum Incoming {
    Request(CliRequest),
    Raise,
}

pub enum Acquire {
    Primary(Primary),
    Forwarded,
}

pub fn acquire(dir: &Path) -> Acquire {
    let _ = std::fs::create_dir_all(dir);
    let lock_path = dir.join("gui.lock");
    let lock = match OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(file) => file,
        Err(err) => {
            eprintln!("could not open {}: {err}", lock_path.display());
            std::process::exit(1);
        }
    };
    match lock.try_lock_exclusive() {
        Ok(()) => {
            let startup = startup_requests();
            for request in &startup {
                write_ack(request);
            }
            let incoming = spawn_listener(dir);
            Acquire::Primary(Primary {
                incoming,
                startup,
                _lock: lock,
            })
        }
        Err(_) => {
            let payload = forward_payload();
            if forward_with_retry(&dir.join("gui.sock"), &payload) {
                Acquire::Forwarded
            } else if cli_request_arg().is_some() {
                eprintln!("could not reach sqlg");
                std::process::exit(5);
            } else {
                eprintln!("sqlg is already running");
                std::process::exit(1);
            }
        }
    }
}

pub fn decode_payload(bytes: &[u8]) -> Option<Incoming> {
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(text).ok()?;
    if value.get("raise").and_then(Value::as_bool) == Some(true) {
        return Some(Incoming::Raise);
    }
    serde_json::from_value::<CliRequest>(value)
        .ok()
        .map(Incoming::Request)
}

pub fn write_ack(request: &CliRequest) {
    if request.ack.is_empty() {
        return;
    }
    let _ = std::fs::write(&request.ack, b"ok");
}

fn startup_requests() -> Vec<CliRequest> {
    let Some(path) = cli_request_arg() else {
        return Vec::new();
    };
    match std::fs::read(&path).and_then(|bytes| {
        serde_json::from_slice::<CliRequest>(&bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }) {
        Ok(request) => vec![request],
        Err(err) => {
            eprintln!("could not read {}: {err}", path.display());
            Vec::new()
        }
    }
}

fn forward_payload() -> Vec<u8> {
    if let Some(path) = cli_request_arg()
        && let Ok(bytes) = std::fs::read(path)
    {
        return bytes;
    }
    br#"{"raise":true}"#.to_vec()
}

fn cli_request_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--cli-request" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn spawn_listener(dir: &Path) -> Receiver<Incoming> {
    let (tx, rx) = mpsc::channel();
    let socket = dir.join("gui.sock");
    let _ = std::fs::remove_file(&socket);
    let listener = match UnixListener::bind(&socket) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("could not listen on {}: {err}", socket.display());
            return rx;
        }
    };
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let Ok(stream) = connection else {
                continue;
            };
            if let Some(incoming) = read_incoming(stream) {
                if let Incoming::Request(request) = &incoming {
                    write_ack(request);
                }
                if tx.send(incoming).is_err() {
                    break;
                }
            }
        }
    });
    rx
}

fn read_incoming(stream: UnixStream) -> Option<Incoming> {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    reader.read_until(b'\n', &mut buf).ok()?;
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    decode_payload(&buf)
}

fn forward_with_retry(path: &Path, payload: &[u8]) -> bool {
    let started = Instant::now();
    loop {
        if forward_once(path, payload).is_ok() {
            return true;
        }
        if started.elapsed() >= Duration::from_secs(5) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn forward_once(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(payload)?;
    if !payload.ends_with(b"\n") {
        stream.write_all(b"\n")?;
    }
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_distinguishes_raise_and_request() {
        assert!(matches!(
            decode_payload(br#"{"raise":true}"#),
            Some(Incoming::Raise)
        ));
        let request =
            br#"{"conn":"dev","sql":"select 1","target":"tab","run":true,"ack":"/tmp/a"}"#;
        match decode_payload(request) {
            Some(Incoming::Request(request)) => {
                assert_eq!(request.conn, "dev");
                assert!(request.run);
            }
            other => panic!("expected request, got {other:?}"),
        }
    }

    #[test]
    fn socket_forwards_one_request() {
        let dir = std::env::temp_dir().join(format!("sqlg-ipc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let incoming = spawn_listener(&dir);
        let ack = dir.join("ack");
        let payload = format!(
            r#"{{"conn":"dev","sql":"select 1","target":"tab","run":false,"ack":"{}"}}"#,
            ack.display()
        );
        forward_once(&dir.join("gui.sock"), payload.as_bytes()).unwrap();
        let incoming = incoming.recv_timeout(Duration::from_secs(2)).unwrap();
        match incoming {
            Incoming::Request(request) => assert_eq!(request.sql, "select 1"),
            Incoming::Raise => panic!("raise"),
        }
        assert!(ack.is_file());
        let _ = std::fs::remove_dir_all(dir);
    }
}
