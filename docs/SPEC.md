# sqlg — Requirements Spec

## Platform
- Desktop app built with **Dioxus 0.7.10** (a desktop webview). Query execution, the connection store, and the `sqlg` command live in separate crates. Dioxus 0.8 is still an alpha, so the app stays on the latest stable release.
- **macOS-first**.
- **MySQL-focused** (other engines later, if ever).

## Windows & tabs
- A window holds multiple tabs; each tab is an independent query workspace (its own connection, editor, results).
- The app **starts with one tab**.
- **⌘T** – New Tab: opens a tab that **inherits the current tab's connection**.
- **⌘N** – New Window: opens a new window with **one fresh tab**.
- Shortcuts are handled by the app: ⌘T new tab, ⌘N new window, ⌘O open, ⌘S save, ⇧⌘S save as, ⌘W close tab, ⇧⌘W close window, ⌘Q quit, ⌘, connections, ⌘↵ run, ⇧⌘↵ statement at the cursor, and ⌘Z / ⇧⌘Z undo and redo in the editor.
- The File menu is a menu inside the window: New Tab, New Window, Open…, Save, Save As…, Save Selection As…, Close Tab, Close Window, Connections…, Install sqlg Command, Uninstall sqlg Command, and Quit.

### Tab title (automatic, not user-editable)
Every tab shows a **connection badge** (a pill with the connection name, colored per connection, e.g. prod in red) followed by a title:
1. **The editor has a comment:** badge + the text of the **first** comment.
   - Supported forms: `-- text`, `# text` (MySQL), `/* text */`. Strip the markers, trim, and collapse whitespace. For a block comment, use its first non-empty line.
   - Truncate to about 30 characters with `…`. The full comment is shown in a tooltip.
   - Comment markers inside string literals don't count (use the editor's SQL syntax tree). MySQL `/*! … */` version comments are ignored.
2. **No comment:** badge + the **query text itself**, with whitespace (including newlines) collapsed to single spaces. It fills the available tab width and is truncated with an ellipsis. The tooltip shows the full first statement.
3. **Empty editor:** just the badge. *Suggestion:* show a muted, italic `New query` placeholder after it.
- Titles update live as the editor changes, debounced (about 300 ms). Running a query doesn't change the title.
- Users can't edit titles directly. To control a tab's name, add or change its first comment.
- *Suggestion:* make the badge color a per-connection setting in the Connections dialog, with red the default for names like `prod`.

## Tab layout
Three columns per tab:
1. **Schema tree** – schemas › tables › columns (with column types).
2. **SQL editor** – a monospace textarea with no line-number gutter and no syntax coloring, plus a Run button.
3. **Results**.

### Layout ("chiclet") control
Segmented control at the top right of the toolbar:
**All | Split | Below | Stack | Editor | Results**. All and Below keep the schema tree. Split is the editor beside the results. Stack is the editor above the results with no schema tree.

## Connections
- **Per-tab connection selector** in the toolbar (e.g. dev / preview / prod), with a status dot.
- An **edit (gear) button** next to it opens the **Connections modal**:
  - List of **named connections** (names are user-editable, e.g. dev, preview, test, prod) with + / − buttons.
  - Form: Name, Host, Port (default 3306), User, Password (masked), Default schema, SSL toggle.
  - Buttons: Test Connection, Cancel, Save.

## Running queries
- The editor can hold **multiple statements**; running produces **stacked result sections, one per statement**.
- Each section header: statement number, short SQL snippet, row count and elapsed time.
- SELECT → results grid. **DML (UPDATE/INSERT/DELETE) → "N rows affected"** (no grid).
- The results column scrolls when the sections overflow.

## Errors & status footer
- Errors appear **only in the failed statement's result section**, which shows a failure marker and the full MySQL error text, e.g. `Error 1146 (42S02): Table 'fleet.TRIPS' doesn't exist`.
- The footer shows a **neutral summary** (no error text), e.g. `3 statements executed` or `2 of 3 statements executed · 1 failed`. The connection name/host and the elapsed time are on the right.

## Files: open and save
- **⌘O Open…** opens a `.sql` file into a **new tab** in the current window, using the current tab's connection. If the file is already open, that tab is focused instead.
- **⌘S Save:** the first save of an unlinked tab shows a **Save As** dialog, and the tab is then **linked** to that file. Later ⌘S saves to it directly.
- **⇧⌘S Save As…** always shows the dialog and re-links the tab to the new file.
- **Unsaved-changes indicator:** a dirty tab shows a **dot in place of the close ✕** (the ✕ appears on hover), like macOS editors. Closing a dirty tab or window asks Save / Don't Save / Cancel, unless session restore preserves the text (see below).
- Saved file tabs still take their **title from the first comment / query** (existing rules). *Suggestion:* show the file path in the tab tooltip (e.g. `… — ~/sql/trip-cleanup.sql`) and set it as the window's represented file (proxy icon in the title bar).
- Files are written as UTF-8 with LF line endings. The file's existing encoding and line endings are preserved when possible. The app watches the linked file and offers to reload if it changes on disk.

### Partial save (selection)
- With text selected, **⌘S** (or **File › Save Selection As…**) saves **only the selection**.
- **Recommendation:** saving a selection **always opens a Save As dialog** and **never re-links the tab or overwrites its file**, so a fragment can't silently replace the full script. Without a selection, ⌘S behaves normally.

## Running queries: selection vs. whole editor
- **⌘↵ / Run** with nothing selected runs the **whole editor** (all statements, stacked results).
- With text **selected**, it runs **only the selection**. The selection may contain several statements, which produce stacked results.
- The Run button stays **Run**. With a selection, ⌘↵ and Run execute that selection.
- *Suggestion:* **⇧⌘↵** runs the **statement under the cursor**, with statement boundaries from the parser.

## Session restore
- The app saves the session **on quit and periodically** (e.g. every 30 s and on significant changes, debounced) and **restores it on relaunch**.
- Persisted:
  - **windows:** position, size, full-screen state, order, and which window is frontmost
  - **tabs per window:** order, active tab, connection name, **editor text (including unsaved edits)**, cursor and selection, scroll position, layout chiclet, linked file path and a dirty flag
  - **theme**
- **Not persisted:** result sets. Queries are **never re-run automatically**. Restored tabs show a **"Run to see results"** placeholder in the results column.
- Stored at `~/Library/Application Support/<bundle-id>/session.json`, written atomically (temp file + rename). **No passwords or credentials.** Connections are referenced by name, and a tab whose connection was deleted shows a "connection missing" badge.
- If a linked file changed on disk while the app was closed, keep the restored (unsaved) text and show a notice offering to reload.
- Restored windows are clamped to the visible screens if the display layout changed.

## Schema tree visuals
- **Chevrons:** thin, SF Symbols-style, rotating from right (collapsed) to down (expanded) with a short animation.
- **Type icons:**
  - schema: database cylinder
  - table: grid
  - columns, by kind:
    - **primary key:** key icon (gold)
    - **foreign key:** link icon (purple)
    - **numeric:** `#`
    - **text:** `Aa`
    - **date/time:** calendar
    - **enum/bool:** toggle
    - other types: a neutral dot
- **Type labels** are muted, monospace and right-aligned.
- **Thin indent guide lines** for each nesting level.
- Mockup: `main.png` / `dark.png`.

## Live checking (errors and typos while typing)
Diagnostics are listed under the editor, up to eight at a time, with the line and a button for each quick fix. The editor does not draw wavy underlines or gutter markers. Mockup: `lint.png` (`#lint` view).

Three layers, all producing the same diagnostic format (`{from, to, severity, message, actions}`):
1. **Syntax:** the `sqlparser` crate with the **MySQL dialect**, run in the app. It runs on every edit, debounced (~250 ms), and maps parser errors to the statement and position.
2. **Schema-aware identifiers:** check against the schema already loaded for autocomplete (from `information_schema`):
   - unknown schema, table or column
   - table aliases resolved (`d.emial` → `DRIVER.emial`), including CTE and derived-table names
   - a **"Did you mean …?"** quick fix using Levenshtein / Damerau distance (e.g. distance ≤ 2), offering the closest matches.
   Names are compared case-insensitively for columns. Table names follow the server's `lower_case_table_names` setting.
3. **Optional server-side validation:** a debounced `EXPLAIN <statement>` against the server (e.g. ~1 s after typing stops) that catches whatever layers 1–2 miss: privileges, functions, and real MySQL grammar.
   - **Opt-in per connection** (a setting in the Connections dialog), **off by default for prod**.
   - **SELECT only.** Never EXPLAIN DML or DDL. Uses a short timeout, and results are cached per statement text.

**Warnings** (orange, not errors):
- `UPDATE` / `DELETE` with no `WHERE` clause ("will modify every row in TRIP").
- *Suggestions:* `SELECT *` on large tables, `LIMIT` without `ORDER BY`, comparing to `NULL` with `=`.

**Limitations and policy:**
- `sqlparser`'s MySQL dialect doesn't cover every MySQL construct (some vendor extensions, optimizer hints, newer syntax, stored-program bodies, `DELIMITER` scripts). Treat its findings as **advisory**: errors flagged by the checker **never block Run**. The server is the final authority.
- If parsing fails, skip schema checks for that statement rather than reporting cascading errors.
- Checking runs per statement, so one broken statement doesn't hide diagnostics in the others.

## Appearance
- **Theme location:** a theme selector at the **far left of the footer**.
- **Scope:** the theme applies to the **whole app**: every tab in every window.
- **Persistence:** the choice is saved in `session.json` and restored on the next launch.
- **Native chrome:** a dark theme uses the dark window theme, and a light theme uses the light window theme.
- **Implementation:** CSS custom properties on the webview document. There is no syntax coloring to switch.
- There is no Auto / System theme.
- Mockups: `main.png` (Light selected), `dark.png` (`#dark` view, Dark selected).

## Implementation suggestions
- Editor: a textarea. Clicking a schema node expands it. Double-clicking a schema, table, or column inserts its quoted name. There is no inline autocomplete.
- Rust: `sqlx` (MySQL) for queries and `sqlparser` for splitting statements and pre-checking them.
- Results grid: an HTML table. The first 100 rows are drawn, and Show more reveals another 100. A query stops after 1000 rows. Clicking a cell copies it.

## Mockups (`docs/mockups/`)
`main.png`, `connections.png`, `editor-results.png`, `error.png`, `menu.png`, `dark.png`, `lint.png`, `selection.png`, `filemenu.png`, rendered from `index.html` (views: default, `#conn`, `#er`, `#err`, `#menu`, `#dark`, `#lint`, `#selection`, `#filemenu`).

## Editor toolbar buttons
The toolbar holds File, the tab strip, the connection selector, Run, and the layout chiclets. New Tab and New Window are in the File menu and on ⌘T and ⌘N.

## Command-line tool (`sqlg`)
The full reference is in **CLI.md**. In short:
- `sqlg conn add <name> --host --port --user (--password-prompt|--password-stdin) --schema --ssl`
- `sqlg conn update <name> [same flags] [--rename <new>]`
- `sqlg conn remove <name> [--yes]`
- `sqlg conn list [--json]`
- `sqlg query <conn> (-e SQL | -f file | stdin) [--tab (default) | --new-window] [--title name] [--no-run]`. Results open in the GUI. `--tab` uses a new tab in the frontmost window, or a new window if none is open. `--title` inserts `-- <title>` as the first line of the SQL, so the first-comment title rule applies.

**Talking to the GUI:** `sqlg query` writes a mode-0600 JSON request and sends that file to the running app on the Unix socket `<data-dir>/gui.sock`. A lock file keeps a single GUI. If nothing is listening, the CLI starts `sqlg-app --cli-request <path>`. The app writes the ack when it has queued the request, not when the query finishes. There is no deep link.
**Connection store:** `~/Library/Application Support/<bundle-id>/connections.toml` (non-secret fields, written atomically) plus passwords in the macOS Keychain. The GUI reloads the file every few seconds.
**Exit codes:** 0 ok, 1 general, 2 usage, 3 name conflict, 4 not found, 5 GUI unreachable, 6 keychain, 7 config I/O.
**Install:** a separate small `sqlg` binary inside the app bundle, symlinked into `/usr/local/bin` via an "Install 'sqlg' Command in PATH" menu item (like VS Code).

```sh
sqlg conn add dev --host 127.0.0.1 --user root --password-prompt --schema fleet
sqlg query dev -e "SELECT * FROM TRIP LIMIT 50"
sqlg query prod -f monthly.sql --new-window --title 'Monthly report'
```
