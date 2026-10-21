import { useEffect, useRef, useState } from "react";
import { Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, type CliPending } from "./api";
import { useApp } from "./store";
import { makeTab, type QueryEvent, type Tab, type WindowSnapshot } from "./types";
import { isDirty, needsFileSave, selectedText } from "../lib/dirty";
import { layoutMode, paneDefaults, paneMin, paneSize, panes, savedColumnWidths } from "../lib/layout";
import { claimRun } from "../lib/runLock";
import { executionSummary } from "../lib/footer";
import { applyTheme, themeById } from "./themes";
import { ClosePrompt } from "../components/ClosePrompt";
import { ConnectionsModal, type Draft } from "../components/ConnectionsModal";
import { QueryEditor } from "../components/Editor";
import { Footer } from "../components/Footer";
import { SettingsModal } from "../components/SettingsModal";
import { Results } from "../components/Results";
import { SchemaTree, type SchemaPhase } from "../components/SchemaTree";
import { TabStrip } from "../components/TabStrip";
import { PaneSplit } from "../components/PaneSplit";
import { Toolbar } from "../components/Toolbar";
import { WindowEdges } from "../components/WindowEdges";

const filters = [{ name: "SQL", extensions: ["sql"] }];
let sessionOpen = false;

function schemaErrorText(error: unknown): string {
  const message = error instanceof Error ? error.message : typeof error === "string" ? error : "Could not load schemas";
  return message.replace(/mysql:\/\/[^/\s]+@/gi, "mysql://");
}

export default function App() {
  const theme = useApp((state) => state.theme);
  const tabs = useApp((state) => state.tabs);
  const activeTabId = useApp((state) => state.activeTabId);
  const connections = useApp((state) => state.connections);
  const schemas = useApp((state) => state.schemas);
  const connectionsOpen = useApp((state) => state.connectionsOpen);
  const settingsOpen = useApp((state) => state.settingsOpen);
  const notice = useApp((state) => state.notice);
  const tab = tabs.find((item) => item.id === activeTabId) ?? tabs[0];
  const connection = connections.find((item) => item.name === tab?.connectionName);
  const layout = panes(tab?.layout ?? "all");
  const [schemaPhase, setSchemaPhase] = useState<Record<string, SchemaPhase>>({});
  const [schemaRetry, setSchemaRetry] = useState(0);
  const [sessionReady, setSessionReady] = useState(false);
  const schemaWidth = useApp((state) => state.schemaWidth);
  const editorWidth = useApp((state) => state.editorWidth);
  const editorHeight = useApp((state) => state.editorHeight);
  const setSchemaWidth = useApp((state) => state.setSchemaWidth);
  const setEditorWidth = useApp((state) => state.setEditorWidth);
  const setEditorHeight = useApp((state) => state.setEditorHeight);
  const bodyRef = useRef<HTMLDivElement>(null);
  const pileRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    let stop = false;
    const unlisten: Array<() => void> = [];
    void boot(() => stop, unlisten).then(
      (ready) => {
        if (!stop && ready) {
          sessionOpen = true;
          setSessionReady(true);
        }
      },
      () => undefined,
    );
    return () => {
      stop = true;
      sessionOpen = false;
      unlisten.forEach((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!sessionReady) return;
    const timer = window.setInterval(() => void persistWindow(), 30_000);
    return () => window.clearInterval(timer);
  }, [sessionReady]);

  useEffect(() => {
    const name = tab?.connectionName;
    if (!name || schemas[name]) return;
    let stale = false;
    setSchemaPhase((current) => ({ ...current, [name]: { kind: "loading" } }));
    void api.loadSchema(name).then(
      (schema) => {
        if (stale) return;
        useApp.getState().setSchema(name, schema);
        setSchemaPhase((current) => {
          if (!(name in current)) return current;
          const next = { ...current };
          delete next[name];
          return next;
        });
      },
      (error: unknown) => {
        if (stale) return;
        setSchemaPhase((current) => ({ ...current, [name]: { kind: "error", message: schemaErrorText(error) } }));
      },
    );
    void api.ping(name).then(
      () => useApp.getState().setStatus(name, "ok"),
      () => useApp.getState().setStatus(name, "down"),
    );
    return () => {
      stale = true;
    };
  }, [tab?.connectionName, schemas, schemaRetry]);

  useEffect(() => {
    // The first paint is one empty tab. Writing it before loadSession returns replaces saved tabs.
    if (!sessionReady) return;
    const timer = window.setTimeout(() => void persistWindow(), 400);
    return () => window.clearTimeout(timer);
  }, [sessionReady, tabs, activeTabId, theme, schemaWidth, editorWidth, editorHeight]);

  if (!tab) return null;

  const rememberColumn = (name: string, width: number | null) => {
    const current = useApp.getState().tabs.find((item) => item.id === tab.id);
    const columnWidths = { ...(current?.columnWidths ?? {}) };
    if (width == null) delete columnWidths[name];
    else columnWidths[name] = width;
    useApp.getState().updateTab(tab.id, { columnWidths });
  };

  return (
    <div className="win">
      <WindowEdges />
      <TabStrip onNew={() => newTab(true)} onClose={(id) => void closeTabs([id])} />
      <Toolbar tab={tab} />
      {notice ? <div className="banner">{notice}</div> : null}
      {tab.diskNotice ? (
        <div className="banner">
          {tab.diskNotice}
          <button type="button" onClick={() => void reloadTab(tab)}>
            Reload
          </button>
          <button type="button" onClick={() => useApp.getState().updateTab(tab.id, { diskNotice: null })}>
            Keep mine
          </button>
        </div>
      ) : null}
      <div className="body" ref={bodyRef}>
        {layout.schema ? (
          <div className="pane schema" style={{ width: schemaWidth }}>
            <SchemaTree
              key={tab.connectionName ?? "none"}
              schema={tab.connectionName ? schemas[tab.connectionName] : undefined}
              defaultSchema={connection?.schema}
              phase={tab.connectionName ? schemaPhase[tab.connectionName] : undefined}
              onRetry={() => setSchemaRetry((attempt) => attempt + 1)}
              onInsert={(text) => insertText(tab, text)}
            />
          </div>
        ) : null}
        {layout.schema && (layout.editor || layout.results) ? (
          <PaneSplit
            axis="x"
            label="Resize schema"
            size={() => schemaWidth}
            min={paneMin.schema}
            max={() => {
              const total = bodyRef.current?.clientWidth ?? 1200;
              const floor = layout.editor && layout.results && !layout.stack ? paneMin.editor + 200 : 220;
              return total - floor;
            }}
            onSize={(next) => {
              setSchemaWidth(next);
              if (!(layout.editor && layout.results) || layout.stack) return;
              const total = bodyRef.current?.clientWidth ?? 0;
              const room = total - 200 - next;
              if (useApp.getState().editorWidth > room) setEditorWidth(Math.max(paneMin.editor, room));
            }}
          />
        ) : null}
        {layout.stack ? (
          <div className="pile" ref={pileRef}>
            {layout.editor ? (
              <div className="pane editor" style={{ height: editorHeight }}>
                <QueryEditor
                  key={tab.id}
                  tab={tab}
                  onRun={(sql, from, to) => void runSql(tab, sql, from, to)}
                  onRunStatement={(cursor) => void runStatement(tab, cursor)}
                  onCancel={() => void cancelRun(tab)}
                  onNewTab={() => newTab(true)}
                  onNewWindow={() => void newWindow()}
                />
              </div>
            ) : null}
            {layout.editor && layout.results ? (
              <PaneSplit
                axis="y"
                label="Resize editor"
                size={() => editorHeight}
                min={paneMin.editorHeight}
                max={() => (pileRef.current?.clientHeight ?? 800) - 160}
                onSize={setEditorHeight}
              />
            ) : null}
            {layout.results ? (
              <div className="pane results">
                <Results
                  sections={tab.results}
                  restored={tab.restored && tab.results.length === 0}
                  columnWidths={tab.columnWidths}
                  onColumnWidth={rememberColumn}
                  schema={tab.connectionName ? schemas[tab.connectionName] : undefined}
                  defaultSchema={connection?.schema}
                />
              </div>
            ) : null}
          </div>
        ) : (
          <>
            {layout.editor ? (
              <div className={layout.results ? "pane editor" : "pane editor grow"} style={layout.results ? { width: editorWidth } : undefined}>
                <QueryEditor
                  key={tab.id}
                  tab={tab}
                  onRun={(sql, from, to) => void runSql(tab, sql, from, to)}
                  onRunStatement={(cursor) => void runStatement(tab, cursor)}
                  onCancel={() => void cancelRun(tab)}
                  onNewTab={() => newTab(true)}
                  onNewWindow={() => void newWindow()}
                />
              </div>
            ) : null}
            {layout.editor && layout.results ? (
              <PaneSplit
                axis="x"
                label="Resize editor"
                size={() => editorWidth}
                min={paneMin.editor}
                max={() => {
                  const total = bodyRef.current?.clientWidth ?? 1200;
                  const schema = layout.schema ? schemaWidth + 12 : 0;
                  return total - schema - 200;
                }}
                onSize={setEditorWidth}
              />
            ) : null}
            {layout.results ? (
              <div className="pane results">
                <Results
                  sections={tab.results}
                  restored={tab.restored && tab.results.length === 0}
                  columnWidths={tab.columnWidths}
                  onColumnWidth={rememberColumn}
                  schema={tab.connectionName ? schemas[tab.connectionName] : undefined}
                  defaultSchema={connection?.schema}
                />
              </div>
            ) : null}
          </>
        )}
      </div>
      <Footer tab={tab} connection={connection} onSettings={() => useApp.getState().setSettingsOpen(true)} />
      {settingsOpen ? (
        <SettingsModal
          theme={theme}
          onTheme={(next) => void changeTheme(next)}
          onClose={() => useApp.getState().setSettingsOpen(false)}
        />
      ) : null}
      {connectionsOpen ? (
        <ConnectionsModal
          connections={connections}
          onClose={() => useApp.getState().setConnectionsOpen(false)}
          onSave={saveConnection}
          onRemove={removeConnection}
          onTest={testConnection}
        />
      ) : null}
      <ClosePrompt />
    </div>
  );
}

async function boot(stopped: () => boolean, unlisten: Array<() => void>): Promise<boolean> {
  if (!("__TAURI_INTERNALS__" in window)) return false;
  const win = getCurrentWindow();
  const label = win.label;
  useApp.getState().setLabel(label);
  let session: Awaited<ReturnType<typeof api.loadSession>>;
  try {
    session = await api.loadSession();
  } catch {
    return false;
  }
  if (stopped()) return false;
  const theme = themeById(session.theme).id;
  useApp.getState().setTheme(theme);
  const scratch = await loadScratch();
  if (stopped()) return false;
  const snapshot = session.windows?.find((window) => window.label === label);
  const scratchTabs = scratch.filter((item) => item.window === label && item.sql);
  const sessionTabs = snapshot?.tabs ?? [];
  const sessionHasWork = sessionTabs.some((item) => (item.sql ?? "").length > 0 || item.filePath);
  const source = !sessionHasWork && scratchTabs.length
    ? scratchTabs.map((item) => ({
        id: item.id,
        connectionName: item.connectionName,
        sql: item.sql,
        cursor: item.cursor ?? 0,
        layout: item.layout,
        filePath: null,
        savedSql: null,
      }))
    : sessionTabs.map((item) => {
        if (item.filePath || item.sql) return item;
        const saved = scratchTabs.find((scratchTab) => scratchTab.id === item.id);
        return saved?.sql ? { ...item, sql: saved.sql, cursor: saved.cursor ?? item.cursor } : item;
      });
  if (source.length) {
    const restored = source.map((item) =>
      makeTab(item.connectionName ?? null, {
        id: item.id,
        sql: item.sql ?? "",
        cursor: item.cursor ?? 0,
        selectionAnchor: "selectionAnchor" in item ? item.selectionAnchor ?? null : null,
        selectionHead: "selectionHead" in item ? item.selectionHead ?? null : null,
        scrollTop: "scrollTop" in item ? item.scrollTop ?? 0 : 0,
        layout: layoutMode(item.layout),
        columnWidths: savedColumnWidths("columnWidths" in item ? item.columnWidths : undefined),
        filePath: item.filePath ?? null,
        savedSql: item.savedSql ?? null,
        newline: ("newline" in item && item.newline) || "\n",
        restored: true,
      }),
    );
    useApp.getState().replaceTabs(restored, snapshot?.activeTab || restored[0].id);
    useApp.getState().setPaneSizes({
      schemaWidth: paneSize(snapshot?.schemaWidth, paneDefaults.schema, paneMin.schema),
      editorWidth: paneSize(snapshot?.editorWidth, paneDefaults.editor, paneMin.editor),
      editorHeight: paneSize(snapshot?.editorHeight, paneDefaults.editorHeight, paneMin.editorHeight),
    });
    for (const item of restored) {
      if (item.filePath) void api.watchFile(item.filePath);
    }
  }
  await refreshConnections();
  unlisten.push(
    await listen<string>("menu", (event) => {
      void onMenu(event.payload);
    }),
  );
  unlisten.push(
    await listen<string>("theme-changed", (event) => {
      useApp.getState().setTheme(themeById(event.payload).id);
    }),
  );
  unlisten.push(await listen("connections-changed", () => void refreshConnections()));
  unlisten.push(await listen("cli-request", () => void takeCli(label)));
  unlisten.push(
    await listen<boolean>("native-close", (event) => {
      if (event.payload) void saveAndAck();
      else void closeWindow();
    }),
  );
  unlisten.push(await listen("prepare-quit", () => void saveAndAck()));
  unlisten.push(
    await listen<{ path: string }>("file-changed", (event) => {
      const path = event.payload.path;
      for (const item of useApp.getState().tabs) {
        if (item.filePath === path) {
          useApp.getState().updateTab(item.id, {
            diskNotice: isDirty(item) ? "This file changed on disk and the tab has unsaved edits." : "This file changed on disk.",
          });
        }
      }
    }),
  );
  await takeCli(label);
  await win.setTitle("sqlg");
  return !stopped();
}

async function refreshConnections() {
  try {
    const connections = await api.listConnections();
    useApp.getState().setConnections(connections);
  } catch {
    /* not running inside the shell */
  }
}

async function takeCli(label: string) {
  const pending = await api.takePending(label);
  if (pending) openCli(pending);
}

function openCli(pending: CliPending) {
  const tab = makeTab(pending.conn, { sql: pending.sql, restored: false });
  useApp.getState().addTab(tab, true);
  if (pending.run) {
    void runSql(tab, pending.sql, 0, pending.sql.length);
  }
}

function newTab(inherit: boolean) {
  const current = currentTab();
  const tab = makeTab(inherit ? current?.connectionName ?? null : null);
  useApp.getState().addTab(tab, true);
}

async function newWindow() {
  await api.createWindow();
}

async function changeTheme(theme: string) {
  useApp.getState().setTheme(theme);
  try {
    await api.setTheme(theme);
  } catch {
    /* preview outside tauri */
  }
}

async function runSql(tab: Tab, sql: string, from: number, to: number) {
  if (!claimRun()) return;
  if (!tab.connectionName) {
    useApp.getState().setNotice("Choose a connection before running.");
    return;
  }
  useApp.getState().setNotice(null);
  useApp.getState().updateTab(tab.id, {
    running: true,
    results: [],
    restored: false,
    footer: { text: "Running…", tone: "run", elapsedMs: null },
    flash: { from, to, token: Date.now() },
  });
  const channel = new Channel<QueryEvent>();
  channel.onmessage = (event) => useApp.getState().applyQuery(tab.id, event);
  try {
    const runId = await api.runQuery(tab.connectionName, sql, channel);
    useApp.getState().updateTab(tab.id, { runId });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    useApp.getState().updateTab(tab.id, {
      running: false,
      results: [{ index: 0, sql, columns: [], rows: [], rowsAffected: null, rowCount: 0, error: message, elapsedMs: null, done: true }],
      footer: { text: executionSummary(1, 1, 1), tone: "fail", elapsedMs: null },
    });
  }
}

async function runStatement(tab: Tab, cursor: number) {
  try {
    const range = await api.statementAt(tab.sql, cursor);
    if (!range) return;
    await runSql(tab, range.sql, range.start, range.end);
  } catch {
    await runSql(tab, tab.sql, 0, tab.sql.length);
  }
}

async function cancelRun(tab: Tab) {
  if (tab.runId) {
    await api.cancelQuery(tab.runId);
    await api.forgetRun(tab.runId);
  }
}

function insertText(tab: Tab, text: string) {
  const sql = tab.sql.length === 0 ? text : `${tab.sql}${tab.sql.endsWith("\n") ? "" : "\n"}${text}`;
  useApp.getState().updateTab(tab.id, { sql, externalRev: tab.externalRev + 1 });
}

async function onMenu(id: string) {
  const tab = currentTab();
  if (!tab) return;
  if (id === "file-new-tab") newTab(true);
  if (id === "file-new-window") await newWindow();
  if (id === "file-open") await openFile(tab);
  if (id === "file-save") await saveTab(tab, selectedText(tab.sql, tab.selectionAnchor, tab.selectionHead) ? "selection" : "save");
  if (id === "file-save-as") await saveTab(tab, "save-as");
  if (id === "file-save-selection") await saveTab(tab, "selection");
  if (id === "file-close-tab") await closeTabs([tab.id]);
  if (id === "file-close-window") await closeWindow();
  if (id === "file-connections") useApp.getState().setConnectionsOpen(true);
  if (id === "view-settings") useApp.getState().setSettingsOpen(true);
  if (id === "query-run") {
    const selection = selectedText(tab.sql, tab.selectionAnchor, tab.selectionHead);
    await runSql(tab, selection || tab.sql, selection ? Math.min(tab.selectionAnchor ?? 0, tab.selectionHead ?? 0) : 0, selection ? Math.max(tab.selectionAnchor ?? 0, tab.selectionHead ?? 0) : tab.sql.length);
  }
  if (id === "query-run-statement") await runStatement(tab, tab.cursor);
  if (id === "query-stop") await cancelRun(tab);
  if (id === "app-install-cli" || id === "app-uninstall-cli") {
    const message = id === "app-install-cli" ? await api.installCli() : await api.uninstallCli();
    useApp.getState().setNotice(message);
  }
}

async function openFile(current: Tab) {
  const picked = await open({ filters, multiple: false });
  if (typeof picked !== "string") return;
  const existing = useApp.getState().tabs.find((tab) => tab.filePath === picked);
  if (existing) {
    useApp.getState().setActive(existing.id);
    return;
  }
  const file = await api.readText(picked);
  const tab = makeTab(current.connectionName, {
    sql: file.text,
    savedSql: file.text,
    filePath: picked,
    newline: file.newline,
    bom: file.bom,
  });
  useApp.getState().addTab(tab, true);
  await api.watchFile(picked);
}

async function saveTab(tab: Tab, mode: "save" | "save-as" | "selection"): Promise<boolean> {
  const selection = selectedText(tab.sql, tab.selectionAnchor, tab.selectionHead);
  if (mode === "selection") {
    if (!selection) {
      useApp.getState().setNotice("Select the text to save.");
      return false;
    }
    const path = await save({ filters, defaultPath: "selection.sql" });
    if (typeof path !== "string") return false;
    await api.writeText(path, selection, "\n", false);
    return true;
  }
  if (mode === "save" && tab.filePath) {
    await api.writeText(tab.filePath, tab.sql, tab.newline || "\n", tab.bom);
    useApp.getState().updateTab(tab.id, { savedSql: tab.sql, diskNotice: null });
    return true;
  }
  const path = await save({ filters, defaultPath: fileName(tab.filePath) });
  if (typeof path !== "string") return false;
  await api.writeText(path, tab.sql, tab.newline || "\n", tab.bom);
  if (tab.filePath && tab.filePath !== path) await api.unwatchFile(tab.filePath);
  await api.watchFile(path);
  useApp.getState().updateTab(tab.id, { filePath: path, savedSql: tab.sql, diskNotice: null });
  return true;
}

async function reloadTab(tab: Tab) {
  if (!tab.filePath) return;
  const file = await api.readText(tab.filePath);
  useApp.getState().updateTab(tab.id, {
    sql: file.text,
    savedSql: file.text,
    newline: file.newline,
    bom: file.bom,
    externalRev: tab.externalRev + 1,
    diskNotice: null,
  });
}

async function closeTabs(ids: string[]) {
  const tabs = useApp.getState().tabs.filter((tab) => ids.includes(tab.id) && needsFileSave(tab));
  if (!(await settleFiles(tabs))) return;
  for (const id of ids) {
    const tab = fresh(id);
    if (tab?.filePath) await api.unwatchFile(tab.filePath);
    useApp.getState().closeTab(id);
  }
}

async function closeWindow() {
  if (!(await settleFiles(useApp.getState().tabs.filter(needsFileSave)))) return;
  await persistWindow();
  await api.permitClose(useApp.getState().label);
}

async function saveAndAck() {
  if (!(await settleFiles(useApp.getState().tabs.filter(needsFileSave)))) {
    try {
      await api.cancelQuit();
    } catch {
      /* the shell is already gone */
    }
    return;
  }
  await persistWindow();
  await api.ackQuit();
}

async function settleFiles(tabs: Tab[]): Promise<boolean> {
  if (!tabs.length) return true;
  try {
    await api.holdQuit(true);
  } catch {
    /* tests and preview have no shell */
  }
  const choice = await useApp.getState().askClose(tabs.map((tab) => tab.id));
  try {
    await api.holdQuit(false);
  } catch {
    /* tests and preview have no shell */
  }
  if (choice === "cancel") return false;
  if (choice === "save") {
    for (const tab of tabs) {
      const saved = await saveTab(fresh(tab.id) ?? tab, "save");
      if (!saved) return false;
    }
    return true;
  }
  for (const tab of tabs) {
    const current = fresh(tab.id) ?? tab;
    useApp.getState().updateTab(current.id, { sql: current.savedSql ?? "", externalRev: current.externalRev + 1 });
  }
  return true;
}

async function loadScratch() {
  try {
    return await api.loadScratch();
  } catch {
    return [];
  }
}

async function persistWindow() {
  if (!sessionOpen || !("__TAURI_INTERNALS__" in window)) return;
  const state = useApp.getState();
  const win = getCurrentWindow();
  let x = 0;
  let y = 0;
  let width = 1440;
  let height = 900;
  let fullscreen = false;
  try {
    const factor = await win.scaleFactor();
    const position = await win.outerPosition();
    const size = await win.outerSize();
    fullscreen = await win.isFullscreen();
    x = position.x / factor;
    y = position.y / factor;
    width = size.width / factor;
    height = size.height / factor;
  } catch {
    /* window metrics are unavailable in tests */
  }
  const snapshot: WindowSnapshot = {
    label: state.label,
    x,
    y,
    width,
    height,
    fullscreen,
    order: 0,
    activeTab: state.activeTabId,
    schemaWidth: state.schemaWidth,
    editorWidth: state.editorWidth,
    editorHeight: state.editorHeight,
    tabs: state.tabs.map((tab) => ({
      id: tab.id,
      connectionName: tab.connectionName,
      sql: tab.sql,
      cursor: tab.cursor,
      selectionAnchor: tab.selectionAnchor,
      selectionHead: tab.selectionHead,
      scrollTop: tab.scrollTop,
      layout: tab.layout,
      columnWidths: savedColumnWidths(tab.columnWidths),
      filePath: tab.filePath,
      savedSql: tab.savedSql,
      newline: tab.newline,
      dirty: isDirty(tab),
    })),
  };
  try {
    await api.upsertWindow(snapshot);
    await api.saveScratch(
      state.label,
      state.tabs
        .filter((tab) => !tab.filePath)
        .map((tab) => ({
          id: tab.id,
          connectionName: tab.connectionName,
          sql: tab.sql,
          cursor: tab.cursor,
          layout: tab.layout,
        })),
    );
  } catch {
    /* ignore when the shell is not up */
  }
}

async function saveConnection(draft: Draft) {
  const port = Number(draft.port) || 3306;
  if (draft.original) {
    await api.updateConnection({
      name: draft.original,
      host: draft.host,
      port,
      user: draft.user,
      password: draft.password || null,
      schema: draft.schema,
      schemaSet: true,
      ssl: draft.ssl,
      explainLint: draft.explainLint,
      color: draft.color,
      colorSet: true,
      rename: draft.name === draft.original ? null : draft.name,
    });
  } else {
    await api.addConnection({
      name: draft.name,
      host: draft.host,
      port,
      user: draft.user,
      password: draft.password,
      schema: draft.schema || null,
      ssl: draft.ssl,
      explainLint: draft.explainLint,
      color: draft.color || null,
    });
  }
  await refreshConnections();
}

async function removeConnection(name: string) {
  await api.removeConnection(name);
  await refreshConnections();
}

async function testConnection(draft: Draft) {
  return api.testConnection({
    name: draft.original,
    host: draft.host,
    port: Number(draft.port) || 3306,
    user: draft.user,
    password: draft.password,
    schema: draft.schema || null,
    ssl: draft.ssl,
  });
}

function currentTab(): Tab | undefined {
  const state = useApp.getState();
  return state.tabs.find((tab) => tab.id === state.activeTabId) ?? state.tabs[0];
}

function fresh(id: string): Tab | undefined {
  return useApp.getState().tabs.find((tab) => tab.id === id);
}

function fileName(path: string | null): string {
  if (!path) return "query.sql";
  const parts = path.split("/");
  return parts[parts.length - 1] || "query.sql";
}
