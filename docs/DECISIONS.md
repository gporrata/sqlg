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
- The window title is `sqlg — {tab label}` when the tab has a label, and `sqlg` when it does not. A tab label is its first comment. The query text is not shown beside the connection selector. An empty editor does not show a tab button. The tab tooltip includes the full title, or the first statement when there is no comment, and the linked path. There is no proxy icon.
- The footer theme selector lists the common editor themes, including Gruvbox, Ayu, and One Dark Pro. Their workbench colors are the Visual Studio Code colors for those themes. Schema names and editor text use `#f4f4f5` on a dark theme and `#1c1c1e` on a light theme. The theme literal stays the original hex. A dark theme sets the native window theme to dark. There is no Auto mode.
- Connections may set `explain_lint` (default false, including names that look like prod) and an optional badge `color`.
- Warnings are: `UPDATE`/`DELETE` without `WHERE`, `SELECT *`, `LIMIT` without `ORDER BY`, and `= NULL`.
- MySQL integration tests are skipped. Docker is not available in this environment.
- Result values use the selected theme's syntax colors: numbers, text, dates, enums, and other types. Column headers use the same high-contrast color as schema column names. A syntax color below 4.5:1 against the editor background is shifted until it is readable. Null stays muted. The theme literal stays the original hex.
- The schema tree and the result grid are not virtualized. The tree expands in place. A result shows 100 rows at a time, and the query stops after 1000 rows. The editor lists at most eight diagnostics and does not draw a line-number gutter.
- Schema names use the high-contrast text color, light on dark themes and dark on light themes. Table names use the theme accent. Alt+F shows a search field under Reload and focuses it. Alt+F again hides the field and clears the query. Each keystroke scrolls the first matching schema, table, or column into view, opening its parents when they are closed. Enter in the field moves to the next match.
- Each tab keeps a separate editor for every connection. Switching connections restores that editor, including unsaved text, and the session stores the editors that are not on screen. Result sets stay with that editor until the app quits.
- Pinning a result keeps it. The next run opens another result beside it. Unpinning a result removes it. An unpinned result is replaced by the next run.
- Rust colors are written as `#rrggbb` or `#rrggbbaa` and passed through the `webcolor!` macro. The macro produces the CSS string and the window background tuple.
- The File menu is a native menu. macOS uses the application menu bar. Linux uses the window menu bar. Reload in the schema pane uses the same compact border as Run and the layout buttons.
- Closing the last window removes it from the session. Quit saves the session, including unsaved editor text, and does not write those buffers to disk.
- `sqlg query` treats stdin as an SQL source only when it is not a terminal and it contains bytes. An immediate EOF is not a second source, so `-e` still works when the parent left stdin as an empty pipe. A non-empty pipe together with `-e` or `-f` is a usage error.
