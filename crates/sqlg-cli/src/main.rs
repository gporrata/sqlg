use std::fs::OpenOptions;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use sqlg_core::{
    BUNDLE_ID, CliRequest, Connection, ConnectionPatch, Secrets, Store, StoreError, data_dir,
    insert_title,
};

#[derive(Parser)]
#[command(
    name = "sqlg",
    about = "Manage sqlg connections and open queries in the app"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage saved connections
    Conn {
        #[command(subcommand)]
        action: ConnCmd,
    },
    /// Open SQL in the sqlg app
    Query {
        conn: String,
        #[arg(short = 'e', long = "execute")]
        execute: Option<String>,
        #[arg(short = 'f', long = "file")]
        file: Option<PathBuf>,
        #[arg(long)]
        tab: bool,
        #[arg(long = "new-window")]
        new_window: bool,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "no-run")]
        no_run: bool,
    },
}

#[derive(Subcommand)]
enum ConnCmd {
    Add {
        name: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 3306)]
        port: u16,
        #[arg(long)]
        user: String,
        #[arg(long)]
        password_prompt: bool,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long)]
        schema: Option<String>,
        #[arg(long, overrides_with = "no_ssl")]
        ssl: bool,
        #[arg(long)]
        no_ssl: bool,
    },
    Update {
        name: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        password_prompt: bool,
        #[arg(long)]
        password_stdin: bool,
        #[arg(long)]
        schema: Option<String>,
        #[arg(long, overrides_with = "no_ssl")]
        ssl: bool,
        #[arg(long)]
        no_ssl: bool,
        #[arg(long)]
        rename: Option<String>,
    },
    Remove {
        name: String,
        #[arg(long)]
        yes: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let code = match Cli::try_parse() {
        Ok(cli) => match cli.command {
            Some(Commands::Conn { action }) => conn(action),
            Some(Commands::Query {
                conn,
                execute,
                file,
                tab,
                new_window,
                title,
                no_run,
            }) => query(conn, execute, file, tab, new_window, title, no_run),
            None => {
                eprintln!("usage: sqlg <conn|query> ...");
                2
            }
        },
        Err(err) => {
            let kind = err.kind();
            err.print().ok();
            if kind == clap::error::ErrorKind::DisplayHelp
                || kind == clap::error::ErrorKind::DisplayVersion
            {
                0
            } else {
                2
            }
        }
    };
    std::process::exit(code);
}

fn store() -> Store {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    Store::open(&dir, Secrets::from_env(&dir))
}

fn conn(action: ConnCmd) -> i32 {
    let store = store();
    let result = match action {
        ConnCmd::Add {
            name,
            host,
            port,
            user,
            password_prompt,
            password_stdin,
            schema,
            ssl,
            no_ssl,
        } => {
            let password = match read_password(password_prompt, password_stdin) {
                Ok(p) => p,
                Err(code) => return code,
            };
            store.add(
                Connection {
                    name,
                    host,
                    port,
                    user,
                    schema,
                    ssl: ssl && !no_ssl,
                    explain_lint: false,
                    color: None,
                },
                &password,
            )
        }
        ConnCmd::Update {
            name,
            host,
            port,
            user,
            password_prompt,
            password_stdin,
            schema,
            ssl,
            no_ssl,
            rename,
        } => {
            let password = if password_prompt || password_stdin {
                match read_password(password_prompt, password_stdin) {
                    Ok(p) => Some(p),
                    Err(code) => return code,
                }
            } else {
                None
            };
            let ssl_flag = if ssl {
                Some(true)
            } else if no_ssl {
                Some(false)
            } else {
                None
            };
            store
                .update(
                    &name,
                    ConnectionPatch {
                        host,
                        port,
                        user,
                        schema: schema.map(Some),
                        ssl: ssl_flag,
                        rename,
                        password,
                        ..ConnectionPatch::default()
                    },
                )
                .map(|_| ())
        }
        ConnCmd::Remove { name, yes } => {
            if !yes {
                if !std::io::stdin().is_terminal() {
                    eprintln!("confirmation required in a non-interactive session; pass --yes");
                    return 2;
                }
                eprint!("Remove connection '{name}'? [y/N] ");
                let _ = std::io::stderr().flush();
                let mut answer = String::new();
                let _ = std::io::stdin().read_line(&mut answer);
                if !answer.trim().eq_ignore_ascii_case("y") {
                    println!("not removed");
                    return 0;
                }
            }
            store.remove(&name)
        }
        ConnCmd::List { json } => return list(&store, json),
    };
    match result {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            err.exit_code()
        }
    }
}

fn list(store: &Store, json: bool) -> i32 {
    match store.list() {
        Ok(connections) => {
            if json {
                let public: Vec<_> = connections
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "name": c.name,
                            "host": c.host,
                            "port": c.port,
                            "user": c.user,
                            "schema": c.schema,
                            "ssl": c.ssl,
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&public).unwrap_or_else(|_| "[]".into())
                );
            } else if connections.is_empty() {
                println!("NAME     HOST                     PORT  USER      SCHEMA  SSL");
            } else {
                println!("NAME     HOST                     PORT  USER      SCHEMA  SSL");
                for c in &connections {
                    println!(
                        "{:<8} {:<24} {:<5} {:<9} {:<7} {}",
                        c.name,
                        c.host,
                        c.port,
                        c.user,
                        c.schema.as_deref().unwrap_or(""),
                        if c.ssl { "yes" } else { "no" }
                    );
                }
            }
            0
        }
        Err(err) => {
            eprintln!("{err}");
            err.exit_code()
        }
    }
}

/// Non-empty stdin is an SQL source when it is not a terminal.
/// An immediate EOF (an unused pipe) is not a source.
fn read_piped_stdin() -> Result<Option<String>, i32> {
    if std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).map_err(|err| {
        eprintln!("could not read stdin: {err}");
        1
    })?;
    if buf.is_empty() {
        Ok(None)
    } else {
        Ok(Some(buf))
    }
}

fn read_password(prompt: bool, stdin: bool) -> Result<String, i32> {
    if prompt == stdin {
        eprintln!("pass exactly one of --password-prompt or --password-stdin");
        return Err(2);
    }
    if prompt {
        rpassword::prompt_password("Password: ").map_err(|err| {
            eprintln!("could not read password: {err}");
            1
        })
    } else {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).map_err(|err| {
            eprintln!("could not read stdin: {err}");
            1
        })?;
        Ok(buf.trim_end_matches(['\n', '\r']).to_string())
    }
}

fn query(
    conn: String,
    execute: Option<String>,
    file: Option<PathBuf>,
    tab: bool,
    new_window: bool,
    title: Option<String>,
    no_run: bool,
) -> i32 {
    if tab && new_window {
        eprintln!("pass only one of --tab or --new-window");
        return 2;
    }
    let stdin_sql = match read_piped_stdin() {
        Ok(sql) => sql,
        Err(code) => return code,
    };
    let sources = [execute.is_some(), file.is_some(), stdin_sql.is_some()]
        .into_iter()
        .filter(|v| *v)
        .count();
    if sources != 1 {
        eprintln!("provide SQL with exactly one of -e, -f, or stdin");
        return 2;
    }
    let sql = if let Some(sql) = execute {
        sql
    } else if let Some(path) = file {
        match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("could not read {}: {err}", path.display());
                return 1;
            }
        }
    } else {
        stdin_sql.unwrap_or_default()
    };
    let store = store();
    if let Err(err) = store.get(&conn) {
        eprintln!("{err}");
        return match err {
            StoreError::NotFound(_) => 4,
            other => other.exit_code(),
        };
    }
    let sql = match title {
        Some(title) => insert_title(&sql, &title),
        None => sql,
    };
    let ack = std::env::temp_dir().join(format!("sqlg-ack-{}", uuid::Uuid::new_v4()));
    let req_path = std::env::temp_dir().join(format!("sqlg-req-{}", uuid::Uuid::new_v4()));
    let req = CliRequest {
        conn,
        sql,
        target: if new_window { "new-window" } else { "tab" }.into(),
        title: None,
        run: !no_run,
        ack: ack.display().to_string(),
    };
    if let Err(err) = write_private(&req_path, &serde_json::to_vec(&req).unwrap_or_default()) {
        eprintln!("could not write request: {err}");
        return 1;
    }
    let status = Command::new("open")
        .args([
            "-n",
            "-b",
            BUNDLE_ID,
            "--args",
            "--cli-request",
            &req_path.display().to_string(),
        ])
        .status();
    if status.map(|s| !s.success()).unwrap_or(true) {
        eprintln!("could not launch {BUNDLE_ID}");
        let _ = std::fs::remove_file(&req_path);
        return 5;
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if ack.exists() {
            let _ = std::fs::remove_file(&ack);
            let _ = std::fs::remove_file(&req_path);
            return 0;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    eprintln!("timed out waiting for sqlg");
    let _ = std::fs::remove_file(&req_path);
    5
}

fn write_private(path: &PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}
