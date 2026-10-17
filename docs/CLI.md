# `sqlg` command-line reference

> The app and its CLI are both named `sqlg`.

`sqlg` lets you manage sqlg's saved connections and open queries in the GUI from a terminal. It uses the same connection store as the app, so changes made in either place appear in the other right away.

## Install
- **App menu › Install 'sqlg' Command in PATH…** (like VS Code's "Install 'code' command"). This symlinks `/usr/local/bin/sqlg` → `sqlg.app/Contents/MacOS/sqlg`, asking for an admin password if needed. It falls back to `~/.local/bin` when `/usr/local/bin` isn't writable.
- `sqlg` is a **small separate Rust binary** shipped inside the app bundle (it's signed and notarized with the app). It starts fast, has no webview dependency, and handles the `conn` commands on its own. `query` hands off to the GUI.
- To uninstall, use the matching menu item, or run `rm /usr/local/bin/sqlg`.

## Commands

### `sqlg conn add <name>`
```
sqlg conn add <name> --host <host> [--port 3306] --user <user>
                     (--password-prompt | --password-stdin) [--schema <db>] [--ssl | --no-ssl]
```
- Fails with exit code 3 if `<name>` already exists.
- The password is never accepted as a flag value, so it can't leak into shell history or `ps`. The CLI stores it in the macOS Keychain.

### `sqlg conn update <name>`
```
sqlg conn update <name> [--host H] [--port P] [--user U]
                        [--password-prompt | --password-stdin] [--schema S] [--ssl | --no-ssl]
                        [--rename <new-name>]
```
Only the flags you give are changed. `--rename` also moves the Keychain entry and updates any open GUI tabs that use the connection. Connection badges in open tabs update to the new name.

### `sqlg conn remove <name> [--yes]`
Asks for confirmation unless `--yes` is given. In a non-interactive session without `--yes` it exits with code 2. It deletes the config entry and the Keychain item.

### `sqlg conn list [--json]`
```
NAME     HOST                     PORT  USER      SCHEMA  SSL
dev      127.0.0.1                3306  root      fleet   no
preview  preview-db.internal      3306  app_ro    fleet   yes
prod     prod-db.internal         3306  app_ro    fleet   yes
```
Passwords are never printed. `--json` prints machine-readable output.

### `sqlg query <conn>`
```
sqlg query <conn> (-e '<sql>' | -f <file.sql> | < stdin)
                  [--tab | --new-window] [--title '<name>'] [--no-run]
```
- Opens the SQL in the GUI and runs it there. Results appear in the GUI, not in the terminal.
- `--tab` (the default) opens a new tab in the frontmost window, or a new window if none is open. `--new-window` always opens a new window with one tab.
- `--title '<name>'` inserts `-- <name>` as the **first line of the SQL**. The tab title then follows the normal rule (first comment becomes the title). Newlines in the value are replaced with spaces. If the SQL already starts with comments, the inserted line comes first, so it wins. Without `--title`, the title comes from the SQL's own first comment, or else from the collapsed query text. Every tab also shows the connection badge.
- `--no-run` loads the SQL into the editor without running it. This is useful for `prod`.
- SQL is read from `-e`, then `-f`, then stdin (when stdin isn't a TTY). Giving more than one source is a usage error.
- The command returns once the GUI has accepted the request. It doesn't wait for the query to finish.

## Exit codes
| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General or unexpected error |
| 2 | Usage error (bad flags, no SQL given, confirmation needed in a non-interactive session) |
| 3 | Connection name conflict (`add` with an existing name, or `--rename` to an existing name) |
| 4 | Connection not found |
| 5 | Couldn't reach or launch the GUI (timed out) |
| 6 | Keychain or credential store error |
| 7 | Config file couldn't be read or written (parse error, permissions) |

## Examples
```sh
# Add dev, prompting for the password
sqlg conn add dev --host 127.0.0.1 --user root --password-prompt --schema fleet

# Add prod from a secret manager, with SSL
op read op://db/prod/password | sqlg conn add prod --host prod-db.internal \
    --user app_ro --password-stdin --schema fleet --ssl

# Rename test to staging and change its host
sqlg conn update test --rename staging --host staging-db.internal

# Remove without a prompt (scripts)
sqlg conn remove preview --yes

# List connections as JSON for scripting
sqlg conn list --json | jq -r '.[].name'

# Run an inline query in a new tab of the front window
sqlg query dev -e "SELECT * FROM TRIP WHERE status = 'in_progress'"

# Run a file in a new window, titled via an inserted "-- Monthly report" comment
sqlg query prod -f monthly.sql --new-window --title 'Monthly report'

# Pipe SQL in, and load it without running it
cat cleanup.sql | sqlg query prod --no-run --title 'Trip cleanup'
```

## How it works
- **Talking to the GUI.** `sqlg query` packages the request as JSON (`{conn, sql, target, title, run}`) and hands it to the app:
  1. **Primary:** the app uses `tauri-plugin-single-instance`. The CLI runs `open -b <bundle-id> --args --cli-request <path-to-temp-request.json>`. If the app is already running, the plugin passes the arguments to that instance and the second process exits. If it isn't running, macOS launches it and it handles the request on startup. Large SQL travels in a temp file (mode 0600, deleted after reading) to avoid argument-length limits.
  2. **Alternative/fallback:** a `sqlglient://query?req=<temp-file-id>` deep link via `tauri-plugin-deep-link`. The SQL is never put in the URL itself.
  The app replies by writing an ack file, or the CLI checks for the app's ready socket. If nothing arrives within 10 s, the CLI exits with code 5.
- **Shared connection store.**
  - `~/Library/Application Support/<bundle-id>/connections.toml` holds the non-secret fields: name, host, port, user, schema, ssl. It's written atomically (temp file + rename), with an advisory file lock while writing.
  - Passwords live in the **macOS Keychain** via the `keyring` crate, under service `<bundle-id>` and account `conn:<name>`.
  - The GUI watches the file with the `notify` crate and reloads it, so a CLI `add`/`update`/`remove` shows up live in the connection selector and the Connections dialog.
  ```toml
  [[connection]]
  name = "dev"
  host = "127.0.0.1"
  port = 3306
  user = "root"
  schema = "fleet"
  ssl = false
  ```
