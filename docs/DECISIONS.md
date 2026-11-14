# Decisions

Choices made where the spec left room, or where the environment differed from the handoff notes.

- The archive on disk was `~/sqlg-handoff.zip`, not `~/sql-handoff.zip`.
- The orphan `master` branch already had one empty commit so the branch would persist. Electron leftovers were not part of this tree. `master_using_electron` and `origin/master` were left untouched. A fast-forward push of this `master` is not possible.
- Bundle id is `dev.sqlg.app`. The id stays, because the data directory and Keychain service are named from it. `SQLG_DATA_DIR` and `SQLG_SECRET_BACKEND` exist only so tests can avoid the login keychain. They are not a user-facing feature.
- The desktop UI is Dioxus 0.7.10. Dioxus 0.8.0-alpha.3 is newer and unpublished as a stable release, so it is not used.
- Lint runs in the app process. The sqlparser crate is not compiled to WASM.
- The `sqlglient://` deep link is not implemented. The CLI sends the request to `<data-dir>/gui.sock`. If the app is not running, the CLI starts `sqlg-app --cli-request <path>`. A second launch holds no GUI lock and forwards the request to the first process.
- The CLI writes the ack when the running app has stored the request, not when the query finishes.
- With a selection, ⌘S is Save Selection As: it always shows a dialog and never relinks the tab. ⇧⌘↵ runs the statement under the cursor.
- Execution stops at the first failed statement. The footer stays a neutral count.
- New files are UTF-8 with LF. An existing UTF-8 BOM and CRLF line endings are preserved. Other encodings are rejected.
- The window title is `sqlg — {tab label}`. The tab tooltip includes the full title and the linked path. There is no proxy icon.
- The footer theme selector lists the common editor themes, including Gruvbox, Ayu, and One Dark Pro. Their colors are the Visual Studio Code workbench colors for those themes. A dark theme sets the native window theme to dark. There is no Auto mode.
- Connections may set `explain_lint` (default false, including names that look like prod) and an optional badge `color`.
- Warnings are: `UPDATE`/`DELETE` without `WHERE`, `SELECT *`, `LIMIT` without `ORDER BY`, and `= NULL`.
- MySQL integration tests are skipped. Docker is not available in this environment.
- The schema tree and the result grid are not virtualized. The tree expands in place. A result shows 100 rows at a time, and the query stops after 1000 rows. The editor lists at most eight diagnostics and does not draw a line-number gutter.
- Closing the last window removes it from the session. Quit saves the session, including unsaved editor text, and does not write those buffers to disk.
- `sqlg query` treats stdin as an SQL source only when it is not a terminal and it contains bytes. An immediate EOF is not a second source, so `-e` still works when the parent left stdin as an empty pipe. A non-empty pipe together with `-e` or `-f` is a usage error.
