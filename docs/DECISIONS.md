# Decisions

Choices made where the spec left room, or where the environment differed from the handoff notes.

- The archive on disk was `~/sqlg-handoff.zip`, not `~/sql-handoff.zip`.
- The orphan `master` branch already had one empty commit so the branch would persist. Electron leftovers were not part of this tree. `master_using_electron` and `origin/master` were left untouched. A fast-forward push of this `master` is not possible.
- Bundle id is `dev.sqlg.app`. Tauri warns that an identifier ending in `.app` collides with the macOS bundle extension. The id stays, because the data directory and Keychain service are named from it. `SQLG_DATA_DIR` and `SQLG_SECRET_BACKEND` exist only so tests can avoid the login keychain. They are not a user-facing feature.
- Lint runs as a Tauri command. The sqlparser crate is not compiled to WASM.
- The `sqlglient://` deep link is not implemented. The CLI launches the app with `open -n -b dev.sqlg.app --args --cli-request <path>` so `tauri-plugin-single-instance` can forward a second process. A plain `open -b` does not deliver `--args` to an app that is already running.
- The CLI writes the ack when the running app has stored the request, not when the query finishes.
- With a selection, ⌘S is Save Selection As: it always shows a dialog and never relinks the tab. ⇧⌘↵ runs the statement under the cursor.
- Execution stops at the first failed statement. The footer stays a neutral count.
- New files are UTF-8 with LF. An existing UTF-8 BOM and CRLF line endings are preserved. Other encodings are rejected.
- Tauri's public API does not set a represented-file proxy icon. The window title stays `sqlg`, and the tab tooltip includes the linked path.
- Theme is Light or Dark. The native window theme follows that choice. There is no Auto mode.
- Connections may set `explain_lint` (default false, including names that look like prod) and an optional badge `color`.
- Warnings are: `UPDATE`/`DELETE` without `WHERE`, `SELECT *`, `LIMIT` without `ORDER BY`, and `= NULL`.
- MySQL integration tests are skipped. Docker is not available in this environment.
- Schema rows are virtualized whenever the tree is shown. Results grids are virtualized the same way.
- Closing the last window removes it from the session. Quit saves the session, including unsaved editor text, and does not write those buffers to disk.
- `sqlg query` treats stdin as an SQL source only when it is not a terminal and it contains bytes. An immediate EOF is not a second source, so `-e` still works when the parent left stdin as an empty pipe. A non-empty pipe together with `-e` or `-f` is a usage error.
