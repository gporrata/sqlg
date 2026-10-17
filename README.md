# sqlg

macOS MySQL client. The window is a Tauri 2 shell around a React editor. A companion `sqlg` command manages connections and opens queries in the app.

Passwords are stored in the macOS Keychain. Connection settings live in `~/Library/Application Support/dev.sqlg.app/connections.toml`. The session file next to it restores windows and editor text. It does not store passwords or result sets.

```sh
pnpm install
pnpm tauri dev
```

```sh
cargo test --workspace
pnpm test
pnpm build
```

Release build, local install:

```sh
pnpm tauri build
cp -R target/release/bundle/macos/sqlg.app /Applications/sqlg.app
```

The app is unsigned. If Gatekeeper blocks it:

```sh
xattr -dr com.apple.quarantine /Applications/sqlg.app
```

Install the command from the app menu, or symlink the bundled binary. The GUI executable is `sqlg-app`. The `sqlg` command is the sidecar next to it.

```sh
ln -sfn /Applications/sqlg.app/Contents/MacOS/sqlg /usr/local/bin/sqlg
```

If `/usr/local/bin` is not writable, the same link works in `~/.local/bin`.

Behavior is specified in `docs/SPEC.md`. The command is specified in `docs/CLI.md`. Ambiguities are recorded in `docs/DECISIONS.md`.
