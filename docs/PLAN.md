# sqlg — Implementation Plan

Repo: `~/personal/sqlg` (GitHub remote `origin`). The app and CLI are both named **sqlg**.
Read `docs/SPEC.md`, `docs/CLI.md` and `docs/mockups/*.png` first. Implement milestone by milestone. After each milestone: build, run the tests, and **commit**.

## Prerequisites (check them; install anything missing)
```sh
xcode-select --install                     # Xcode Command Line Tools (skip if `xcode-select -p` works)
curl https://sh.rustup.rs -sSf | sh        # Rust via rustup (stable); then: rustup update
brew install node@22 pnpm gh               # Node LTS + pnpm + GitHub CLI (or use corepack: corepack enable pnpm)
cargo install tauri-cli --version "^2"     # gives `cargo tauri`; the pnpm script `pnpm tauri` also works
gh auth status || gh auth login            # needed for pushing to GitHub
```
Versions: Tauri **2.x**, Node ≥ 20 LTS, Rust stable.

## Stack decision
- **Frontend: React + TypeScript + Vite.** React is the framework coding agents know best, and it has the richest ecosystem for what we need (`@uiw/react-codemirror` or raw CodeMirror 6, TanStack Table + TanStack Virtual). Svelte or SolidJS would be lighter, but the runtime cost doesn't matter here and agent familiarity does.
- **State:** Zustand (a small store per window). **Styling:** plain CSS with CSS variables (light/dark themes) to match the mockups.
- **Backend:** Rust, Tauri 2, `sqlx` (mysql, runtime-tokio, rustls), `sqlparser` (MySQL dialect), `keyring`, `toml`/`serde`, `notify`, `tauri-plugin-single-instance`, `tauri-plugin-dialog`, `tauri-plugin-fs`, `tauri-plugin-store` (or our own JSON), `tauri-plugin-deep-link` (fallback).

## Suggested repo layout
```
sqlg/
├─ Cargo.toml                 # [workspace] members = ["src-tauri", "crates/*"]
├─ package.json  pnpm-lock.yaml  vite.config.ts  tsconfig.json  index.html
├─ src/                       # React frontend
│  ├─ app/ (App.tsx, store.ts, theme.css)
│  ├─ components/ (TabStrip, Toolbar, LayoutChiclets, SchemaTree, Editor, Results, Footer, ConnectionsModal)
│  ├─ lib/ (tauri.ts invoke wrappers, titles.ts, lint/, session.ts)
│  └─ __tests__/
├─ src-tauri/                 # Tauri app crate (bin "sqlg-app", bundle name "sqlg")
│  ├─ src/ (main.rs, commands/, db/, menu.rs, session.rs, cli_bridge.rs)
│  ├─ capabilities/  icons/  tauri.conf.json
├─ crates/
│  ├─ sqlg-core/              # shared: connection model, TOML store, keyring, paths, request format
│  └─ sqlg-cli/               # `sqlg` CLI binary (clap); depends on sqlg-core
├─ docs/ (SPEC.md, CLI.md, PLAN.md, mockups/)
└─ .github/workflows/ci.yml   # optional: cargo test + pnpm test on macos-latest
```
Bundle id: `com.<owner>.sqlg` (pick one, e.g. `dev.sqlg.app`). Data dir: `~/Library/Application Support/<bundle-id>/`.

---

## Step 0 — Check the empty repo and scaffold
`~/personal/sqlg` is an **empty git repo**: initialized, possibly with a remote, and no code yet.
```sh
cd ~/personal/sqlg
git status                 # expect "No commits yet" and nothing to commit
ls -A                      # expect only .git (maybe a README/.gitignore); if real code is present, stop and ask
git branch --show-current  # note the current branch name; all work happens here
git remote -v              # if no remote is configured, ask the owner for the URL before the first push
```
Scaffold Tauri 2 + React + TypeScript in place, and add the docs:
```sh
pnpm create tauri-app@latest . --template react-ts --manager pnpm --identifier dev.sqlg.app
#   (if it refuses because the directory isn't empty, e.g. a README exists: scaffold in /tmp/sqlg-scaffold and copy the files in)
mkdir -p docs && cp -R /path/to/sqlg-handoff/docs/* docs/   # if the docs aren't already present
pnpm install && pnpm tauri build --debug
git add -A && git commit -m "M0: Tauri 2 + React scaffold and sqlg design docs"
git push -u origin "$(git branch --show-current)"            # first push sets the upstream
```
**Acceptance:** the scaffold builds, `docs/` is committed, and the branch is pushed with an upstream set (or the owner has been asked for a remote).

---

## M1 — App shell: window, tabs, layout, footer, theme
- Tauri 2 window (title-bar style overlay/transparent so the custom tab strip sits next to the traffic lights), React app.
- Tab strip (starts with one tab, `+` button), toolbar (connection selector placeholder, gear button, **layout chiclets**: All | Editor + Results | Editor only | Results only), three resizable columns, footer with the **light/dark toggle at the far left** and the status area on the right.
- Editor header with **New Tab ⌘T**, **New Window ⌘N** and **Run ⌘↵** buttons (wired up later).
- Theme via CSS variables, persisted (tauri-plugin-store), broadcast to all windows, `window.set_theme()` to match.
- **Accept:** the UI matches `main.png` / `dark.png` with fake data. The chiclets show and hide columns. The theme toggles and survives a restart.
- **Tests:** Vitest + React Testing Library for the layout reducer (chiclet → visible columns) and the theme store. `cargo test` passes (empty is fine).

## M2 — Connections store + modal + pools
- `sqlg-core`: a `Connection {name, host, port, user, schema, ssl}` model, the TOML store at `~/Library/Application Support/<bundle-id>/connections.toml` (atomic write + file lock), and Keychain via `keyring` (service = bundle id, account = `conn:<name>`). Rename moves the Keychain entry.
- Tauri commands: `list_connections`, `save_connection`, `delete_connection`, `rename_connection`, `test_connection`.
- The GUI watches the file (`notify`) and emits `connections-changed`.
- Connections modal per `connections.png` (list with + / −, form, SSL toggle, Test / Cancel / Save). A per-tab connection selector with a colored dot.
- `sqlx::MySqlPool` per connection, created lazily, with a small max pool size and a timeout.
- **Accept:** CRUD works and persists. Passwords aren't in the TOML. An external edit to the TOML updates the UI live. Test Connection reports the server version or the error.
- **Tests:** `sqlg-core` unit tests on a temp dir (TOML round trip, rename, delete, atomic write), with a mock keyring backend. An integration test against MySQL in Docker (`mysql:8`) if Docker is available, skipped otherwise.

## M3 — Schema tree
- Load schemas › tables › columns from `information_schema` (`SCHEMATA`, `TABLES`, `COLUMNS`, `KEY_COLUMN_USAGE` for PK/FK) with lazy expansion and a cache per connection, plus a refresh action.
- Visuals per the spec and `main.png`: thin rotating chevrons, database / table icons, column icons (PK key, FK link, `#` numeric, `Aa` text, calendar for date/time, toggle for enum/bool, neutral dot otherwise), right-aligned muted types, indent guides.
- **Accept:** the tree for a real DB matches the mockup style, and expanding is fast on large schemas (virtualize if > 500 nodes).
- **Tests:** a unit test mapping MySQL data types to icon kinds, and a snapshot test of the tree rendering.

## M4 — Editor
- CodeMirror 6, `@codemirror/lang-sql` with the `MySQL` dialect, and **schema autocomplete** fed from the M3 cache (`schema` option, updated when the connection or schema changes). Line numbers, light and dark themes, ⌘↵ keybinding.
- **Accept:** syntax coloring as in the mockups. Typing `TRIP.` suggests columns.
- **Tests:** a unit test of the schema → completion-config transform.

## M5 — Query execution
- Statement splitting with `sqlparser` (MySQL dialect), plus a fallback splitter that respects quotes and comments (for statements sqlparser can't parse). The frontend sends text plus its offsets.
- **Run all** (no selection) or **Run selection** (only the selected range, possibly several statements). The button label becomes "Run Selection ⌘↵". The executed range flashes briefly.
- Rows are streamed from Rust in batches (Tauri channel/events) into **stacked result sections**, one per statement. Each header shows the number, a snippet and rows/time. Grids use TanStack Table + TanStack Virtual (zebra rows, sticky header). DML shows "N rows affected" (no grid).
- **Errors only in the failed statement's section** (full MySQL code, SQLSTATE and message). Execution stops at the first failure. The footer shows a neutral summary (`3 statements executed` / `2 of 3 statements executed · 1 failed`) with the elapsed time on the right.
- **Cancel:** a Stop button / ⌘. that issues `KILL QUERY <connection_id>` on a side connection.
- **Accept:** reproduces `main.png`, `error.png` and `selection.png` behavior on a real DB. 100k rows scroll smoothly. Cancel works.
- **Tests:** Rust tests for splitting (comments, quotes, `DELIMITER` is not supported → one statement), and integration tests on Docker MySQL (SELECT, UPDATE rows affected, error 1146 surfaced per statement).

## M6 — Tab titles + connection badges
- Every tab shows a **connection badge** (color per connection; red by default for `prod`).
- Title = **first comment** (`-- `, `# `, `/* */`; strip, trim, collapse whitespace, first line; ignore markers inside strings and `/*! */`), truncated to about 30 characters with "…". Otherwise the **query text** with whitespace collapsed, filling the tab width with an ellipsis. Empty editor: badge plus a muted "New query". The tooltip shows the full comment or first statement (plus the file path if linked). Debounced ~300 ms. **Not user-editable.**
- **Accept:** matches the tab strip in `main.png`.
- **Tests:** table-driven unit tests for `deriveTitle(text)` covering all comment styles, strings containing `--`, block comments, empty text and whitespace.

## M7 — Live linting
- `@codemirror/lint` with squiggles, gutter markers, hover messages and quick-fix actions.
- Layer 1: syntax via `sqlparser` (a Tauri command, or WASM if it's cheap), debounced ~250 ms.
- Layer 2: schema-aware checks for unknown schema/table/column, with aliases resolved and "Did you mean" (Levenshtein ≤ 2) quick fixes.
- Layer 3 (opt-in per connection, default off for prod, SELECT only): debounced `EXPLAIN`.
- Warnings: UPDATE/DELETE without WHERE. Lint findings **never block Run**.
- **Accept:** matches `lint.png`.
- **Tests:** unit tests for alias resolution, suggestions and warnings; a parser-error position-mapping test.

## M8 — Files
- ⌘O Open (new tab), ⌘S Save (Save As the first time, then linked), ⇧⌘S Save As, **Save Selection As…** (always a dialog, never re-links or overwrites the tab's file). Unsaved dot replaces ✕. A Save / Don't Save / Cancel prompt on closing a tab or window. Watch the linked file for external changes.
- **Accept:** round-trips files. The dirty state is correct. A selection save never touches the linked file.
- **Tests:** unit tests for the dirty-state store, and a Rust test for atomic file writes.

## M9 — Session restore
- `session.json` in Application Support holds windows (frame, order, frontmost), tabs (order, active tab, connection, text including unsaved edits, cursor/selection, scroll, chiclet, file path, dirty flag) and the theme. Saved on quit, every ~30 s and debounced on change, written atomically. **No passwords.** Result sets aren't restored and queries never re-run automatically ("Run to see results").
- **Accept:** quit with 2 windows and 4 tabs (some unsaved), relaunch, and everything is back as it was.
- **Tests:** serialization round trip, plus migration/versioning (`"version": 1`) for missing or extra fields.

## M10 — Menus, shortcuts, single instance, CLI
- Native menu (Tauri menu API) with accelerators. File menu: New Tab `CmdOrCtrl+T`, New Window `CmdOrCtrl+N`, Open… `CmdOrCtrl+O`, Save `CmdOrCtrl+S`, Save As… `Shift+CmdOrCtrl+S`, Save Selection As…, Close Tab `CmdOrCtrl+W`, Close Window `Shift+CmdOrCtrl+W`, Connections… `CmdOrCtrl+,`. Query menu: Run `CmdOrCtrl+Enter`. App menu: **Install 'sqlg' Command in PATH…** and Uninstall. ⌘T inherits the current tab's connection; ⌘N opens a window with one fresh tab.
- `tauri-plugin-single-instance`: handles `--cli-request <file>` (from argv or the forwarded argv) and opens tabs/windows per CLI.md.
- `crates/sqlg-cli` (clap): `sqlg conn add|update|remove|list`, `sqlg query …` per **CLI.md**, with the exit codes from CLI.md. `--title` inserts `-- <title>` as the first line. The `sqlg` binary is bundled into `sqlg.app/Contents/MacOS/sqlg` (Tauri `bundle.macOS` / an `externalBin` sidecar, or a post-build copy).
- **Accept:** all CLI examples in CLI.md work against a running app and a closed one (it launches).
- **Tests:** CLI arg parsing and exit-code tests (`assert_cmd`), and request-file round trip.

## M11 — Build & install locally
```sh
pnpm install && pnpm tauri build            # produces src-tauri/target/release/bundle/macos/sqlg.app
rm -rf /Applications/sqlg.app && cp -R src-tauri/target/release/bundle/macos/sqlg.app /Applications/
sudo ln -sf /Applications/sqlg.app/Contents/MacOS/sqlg /usr/local/bin/sqlg   # or use the in-app Install command
sqlg conn list && open -a sqlg
```
- Local builds are unsigned. If Gatekeeper complains: `xattr -dr com.apple.quarantine /Applications/sqlg.app`.
- **Accept:** the app launches from /Applications and `sqlg query dev -e "SELECT 1"` opens a tab.

## M12 — Push to GitHub
```sh
cargo test --workspace && pnpm test && pnpm tauri build
git push                                     # plain push of the current branch (upstream set in Step 0)
```
- Optional: a CI workflow on `macos-latest` (cargo test, pnpm test, pnpm build).
- **Accept:** the remote branch holds all milestone commits and CI is green (if added).

## Conventions
- One commit (or more) per milestone: `M<n>: <summary>`. Run `cargo fmt`, `cargo clippy -- -D warnings`, `pnpm lint` and the tests before each commit.
- No secrets in the repo, logs or session.json.
- Pushes: `git push -u origin <branch>` the first time, then plain `git push`. Never rewrite history or rename or delete branches.
