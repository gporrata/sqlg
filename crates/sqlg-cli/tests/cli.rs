use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn cmd(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("sqlg").unwrap();
    cmd.env("SQLG_DATA_DIR", dir);
    cmd.env("SQLG_SECRET_BACKEND", "file");
    cmd
}

#[test]
fn usage_and_conflict_and_missing() {
    let dir = tempdir().unwrap();
    cmd(dir.path()).assert().code(2);

    cmd(dir.path())
        .args([
            "conn",
            "add",
            "dev",
            "--host",
            "127.0.0.1",
            "--user",
            "root",
        ])
        .assert()
        .code(2);

    cmd(dir.path())
        .args([
            "conn",
            "add",
            "dev",
            "--host",
            "127.0.0.1",
            "--user",
            "root",
            "--password-stdin",
            "--schema",
            "fleet",
        ])
        .write_stdin("s3cret\n")
        .assert()
        .success();

    let listed = cmd(dir.path())
        .args(["conn", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"dev\""));
    let text = String::from_utf8_lossy(&listed.get_output().stdout);
    assert!(!text.contains("s3cret"));

    cmd(dir.path())
        .args([
            "conn",
            "add",
            "dev",
            "--host",
            "127.0.0.1",
            "--user",
            "root",
            "--password-stdin",
        ])
        .write_stdin("x\n")
        .assert()
        .code(3);

    cmd(dir.path())
        .args(["conn", "remove", "missing", "--yes"])
        .assert()
        .code(4);

    cmd(dir.path())
        .args(["query", "dev", "-e", "select 1", "-f", "also.sql"])
        .assert()
        .code(2);

    cmd(dir.path())
        .args(["query", "dev", "-e", "select 1"])
        .write_stdin("select 2\n")
        .assert()
        .code(2);

    cmd(dir.path())
        .args(["query", "nope", "-e", "select 1"])
        .assert()
        .code(4);
}
