# sqlg

macOS MySQL client. The window is a native [iced](https://iced.rs) app. A companion `sqlg` command manages connections and opens queries in the app.

Passwords are stored in the macOS Keychain. Connection settings live in `~/Library/Application Support/dev.sqlg.app/connections.toml`. The session file next to it restores windows and editor text. It does not store passwords or result sets.

```sh
cargo run -p sqlg-app
```

```sh
cargo test --workspace
```

Release build and local install:

```sh
make install
```

That builds `sqlg-app` and `sqlg`, installs them with `cargo install` into `~/.cargo/bin` when that directory exists, and copies a minimal `sqlg.app` to `/Applications`. The app is unsigned. If Gatekeeper blocks it:

```sh
xattr -dr com.apple.quarantine /Applications/sqlg.app
```

Install the command from the app's File menu, or symlink the bundled binary. The GUI executable is `sqlg-app`. The `sqlg` command is the binary next to it.

```sh
ln -sfn /Applications/sqlg.app/Contents/MacOS/sqlg /usr/local/bin/sqlg
```

If `/usr/local/bin` is not writable, the same link works in `~/.local/bin`.

`sqlg query` sends the request to the running app on a Unix socket. If the app is not running, the command starts `sqlg-app`.

Behavior is specified in `docs/SPEC.md`. The command is specified in `docs/CLI.md`. Ambiguities are recorded in `docs/DECISIONS.md`.
