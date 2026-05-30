import { useEffect, useMemo, useRef, useState } from "react";
import { basicSetup, EditorView } from "codemirror";
import { sql } from "@codemirror/lang-sql";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import { Database, Play, Plus, Settings as SettingsIcon, Trash2, X } from "lucide-react";
import type { Connection, ConnectionInput, QueryResult, Settings, TabName, ThemeName } from "./types";

const themeOptions: { value: ThemeName; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "solarized", label: "Solarized" },
];

const starterSql = `select
  c.id,
  c.name,
  c.email,
  o.id as orders__id,
  o.order_number as orders__order_number,
  o.total as orders__total,
  oi.id as items__id,
  oi.sku as items__sku,
  oi.quantity as items__quantity
from customers c
left join orders o on o.customer_id = c.id
left join order_items oi on oi.order_id = o.id
order by c.id, o.id, oi.id;`;

const api = window.sqlg ?? createBrowserPreviewApi();

function App() {
  const [tab, setTab] = useState<TabName>("connections");
  const [connections, setConnections] = useState<Connection[]>([]);
  const [activeConnectionId, setActiveConnectionId] = useState("");
  const [sqlText, setSqlText] = useState(starterSql);
  const [result, setResult] = useState<QueryResult | null>(null);
  const [footer, setFooter] = useState("Ready");
  const [settings, setSettings] = useState<Settings>({ theme: "system" });
  const [showConnectionModal, setShowConnectionModal] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const editorRef = useRef<EditorView | null>(null);

  const activeConnection = connections.find((connection) => connection.id === activeConnectionId);

  useEffect(() => {
    api
      .listConnections()
      .then((items) => {
        setConnections(items);
        setActiveConnectionId(items[0]?.id ?? "");
      })
      .catch((error) => setFooter(error.message));

    api
      .getSettings()
      .then((stored) => setSettings({ theme: stored.theme ?? "system" }))
      .catch((error) => setFooter(error.message));
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
  }, [settings.theme]);

  async function refreshConnections() {
    const items = await api.listConnections();
    setConnections(items);
    setActiveConnectionId((current) => current || items[0]?.id || "");
  }

  async function executeCurrentSql() {
    if (!activeConnectionId) {
      setFooter("Select a connection before executing SQL");
      return;
    }
    const editor = editorRef.current;
    const state = editor?.state;
    const selection = state ? state.sliceDoc(state.selection.main.from, state.selection.main.to).trim() : "";
    const query = selection || editor?.state.doc.toString() || sqlText;
    try {
      setFooter("Executing query...");
      const nextResult = await api.executeQuery({ connectionId: activeConnectionId, sql: query });
      setResult(nextResult);
      setTab("results");
      setFooter(`${nextResult.rowCount} row(s) returned in ${nextResult.elapsedMs}ms`);
    } catch (error) {
      setFooter(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <main className="app-shell">
      <header className="app-header">
        <div className="brand">
          <Database size={20} />
          <span>sqlg</span>
        </div>
        <nav className="tabs" aria-label="Main tabs">
          <button className={tab === "connections" ? "active" : ""} onClick={() => setTab("connections")}>
            Connections
          </button>
          <button className={tab === "editor" ? "active" : ""} onClick={() => setTab("editor")}>
            SQL Editor
          </button>
          <button className={tab === "results" ? "active" : ""} onClick={() => setTab("results")}>
            Results
          </button>
        </nav>
      </header>

      <section className="workspace">
        {tab === "connections" && (
          <ConnectionsView
            connections={connections}
            activeConnectionId={activeConnectionId}
            onSelect={setActiveConnectionId}
            onAdd={() => setShowConnectionModal(true)}
            onDelete={async (id) => {
              await api.deleteConnection(id);
              await refreshConnections();
              setFooter("Connection deleted");
            }}
            onTest={async (id) => {
              try {
                setFooter("Testing connection...");
                await api.testConnection(id);
                setFooter("Connection succeeded");
              } catch (error) {
                setFooter(error instanceof Error ? error.message : String(error));
              }
            }}
          />
        )}
        {tab === "editor" && (
          <EditorViewPane
            activeConnection={activeConnection}
            sqlText={sqlText}
            settings={settings}
            onChange={setSqlText}
            onExecute={executeCurrentSql}
            editorRef={editorRef}
          />
        )}
        {tab === "results" && <ResultsView result={result} />}
      </section>

      <footer className="footer">
        <span>{footer}</span>
        <button title="Settings" className="icon-button" onClick={() => setShowSettingsModal(true)}>
          <SettingsIcon size={18} />
        </button>
      </footer>

      {showConnectionModal && (
        <ConnectionModal
          onClose={() => setShowConnectionModal(false)}
          onSave={async (connection) => {
            const saved = await api.addConnection(connection);
            await refreshConnections();
            setActiveConnectionId(saved.id);
            setShowConnectionModal(false);
            setFooter("Connection saved");
          }}
        />
      )}
      {showSettingsModal && (
        <SettingsModal
          settings={settings}
          onClose={() => setShowSettingsModal(false)}
          onSave={async (next) => {
            const saved = await api.saveSettings(next);
            setSettings(saved);
            setShowSettingsModal(false);
            setFooter("Settings saved");
          }}
        />
      )}
    </main>
  );
}

function createBrowserPreviewApi(): Window["sqlg"] {
  const fallbackConnection: Connection = {
    id: "browser-preview",
    name: "Browser Preview",
    type: "postgres",
    host: "127.0.0.1",
    port: 55432,
    database: "sqlg",
    user: "sqlg",
    hasPassword: true,
  };

  return {
    async listConnections() {
      const stored = localStorage.getItem("sqlg-preview-connections");
      return stored ? JSON.parse(stored) : [fallbackConnection];
    },
    async addConnection(connection) {
      const saved = { ...connection, id: connection.id || `preview-${Date.now()}`, hasPassword: Boolean(connection.password) };
      const current = await this.listConnections();
      localStorage.setItem("sqlg-preview-connections", JSON.stringify([...current.filter((item) => item.id !== saved.id), saved]));
      return saved;
    },
    async deleteConnection(id) {
      const current = await this.listConnections();
      localStorage.setItem("sqlg-preview-connections", JSON.stringify(current.filter((item) => item.id !== id)));
      return true;
    },
    async testConnection() {
      return true;
    },
    async executeQuery(payload) {
      return {
        rows: [
          { id: 1, name: "Ada Lovelace", email: "ada@example.com", orders__id: 1, orders__order_number: "PG-1001", orders__total: "199.50", items__id: 1, items__sku: "SQL-BOOK", items__quantity: 1 },
          { id: 1, name: "Ada Lovelace", email: "ada@example.com", orders__id: 1, orders__order_number: "PG-1001", orders__total: "199.50", items__id: 2, items__sku: "QUERY-MUG", items__quantity: 2 },
          { id: 2, name: "Grace Hopper", email: "grace@example.com", orders__id: 3, orders__order_number: "PG-1003", orders__total: "75.25", items__id: 4, items__sku: "PLAN-HOODIE", items__quantity: 1 },
        ],
        fields: ["id", "name", "email", "orders__id", "orders__order_number", "orders__total", "items__id", "items__sku", "items__quantity"],
        rowCount: 3,
        connectionName: fallbackConnection.name,
        sql: payload.sql,
        elapsedMs: 18,
        executedAt: new Date().toISOString(),
      };
    },
    async getSettings() {
      const stored = localStorage.getItem("sqlg-preview-settings");
      return stored ? JSON.parse(stored) : { theme: "system" };
    },
    async saveSettings(settings) {
      localStorage.setItem("sqlg-preview-settings", JSON.stringify(settings));
      return settings;
    },
  };
}

function ConnectionsView(props: {
  connections: Connection[];
  activeConnectionId: string;
  onSelect(id: string): void;
  onAdd(): void;
  onDelete(id: string): void;
  onTest(id: string): void;
}) {
  return (
    <div className="pane">
      <div className="pane-toolbar">
        <h1>Connections</h1>
        <button className="primary" onClick={props.onAdd}>
          <Plus size={16} />
          Add Connection
        </button>
      </div>
      <div className="connection-list">
        {props.connections.map((connection) => (
          <article
            className={`connection-row ${connection.id === props.activeConnectionId ? "selected" : ""}`}
            key={connection.id}
            onClick={() => props.onSelect(connection.id)}
          >
            <div>
              <strong>{connection.name}</strong>
              <span>
                {connection.type}://{connection.user}@{connection.host}:{connection.port}/{connection.database}
              </span>
            </div>
            <div className="row-actions">
              <button onClick={(event) => void (event.stopPropagation(), props.onTest(connection.id))}>Test</button>
              <button className="icon-button danger" title="Delete" onClick={(event) => void (event.stopPropagation(), props.onDelete(connection.id))}>
                <Trash2 size={16} />
              </button>
            </div>
          </article>
        ))}
        {props.connections.length === 0 && <p className="empty">No connections yet. Add one for the Docker sample databases.</p>}
      </div>
    </div>
  );
}

function EditorViewPane(props: {
  activeConnection?: Connection;
  sqlText: string;
  settings: Settings;
  onChange(value: string): void;
  onExecute(): void;
  editorRef: React.MutableRefObject<EditorView | null>;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const executeRef = useRef(props.onExecute);

  useEffect(() => {
    executeRef.current = props.onExecute;
  }, [props.onExecute]);

  useEffect(() => {
    if (!hostRef.current) return;
    const view = new EditorView({
      parent: hostRef.current,
      state: EditorState.create({
        doc: props.sqlText,
        extensions: [
          basicSetup,
          sql(),
          props.settings.theme === "dark" || props.settings.theme === "solarized" ? oneDark : [],
          EditorView.updateListener.of((update) => {
            if (update.docChanged) props.onChange(update.state.doc.toString());
          }),
          EditorView.domEventHandlers({
            keydown(event, view) {
              if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "e") {
                event.preventDefault();
                executeRef.current();
                return true;
              }
              return false;
            },
          }),
        ],
      }),
    });
    props.editorRef.current = view;
    return () => {
      view.destroy();
      props.editorRef.current = null;
    };
  }, [props.settings.theme]);

  return (
    <div className="pane editor-pane">
      <div className="pane-toolbar">
        <div>
          <h1>SQL Editor</h1>
          <p>{props.activeConnection ? `Using ${props.activeConnection.name}` : "No connection selected"}</p>
        </div>
        <button className="primary" disabled={!props.activeConnection} onClick={props.onExecute}>
          <Play size={16} />
          Execute
        </button>
      </div>
      <div className="editor-host" ref={hostRef} />
    </div>
  );
}

function ResultsView({ result }: { result: QueryResult | null }) {
  const grouped = useMemo(() => (result ? groupRows(result.rows, result.fields) : []), [result]);
  if (!result) {
    return <div className="pane empty">Execute a query to see hierarchical results.</div>;
  }
  return (
    <div className="pane results-pane">
      <div className="pane-toolbar">
        <div>
          <h1>Results</h1>
          <p>
            {result.connectionName} | {result.rowCount} row(s) | {result.elapsedMs}ms
          </p>
        </div>
      </div>
      <div className="result-tree">
        {grouped.map((group) => (
          <section className="result-group" key={group.key}>
            <header className="result-header">{group.parentTitle}</header>
            <DataTable rows={[group.parent]} />
            {Object.entries(group.children).map(([childName, rows]) => (
              <div className="child-table" key={childName}>
                <h2>{childName}</h2>
                <DataTable rows={rows} />
              </div>
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}

function DataTable({ rows }: { rows: Record<string, unknown>[] }) {
  const columns = Object.keys(rows[0] ?? {});
  if (!rows.length || !columns.length) return <p className="empty compact">No joined rows</p>;
  return (
    <table>
      <thead>
        <tr>{columns.map((column) => <th key={column}>{column}</th>)}</tr>
      </thead>
      <tbody>
        {rows.map((row, index) => (
          <tr key={index}>
            {columns.map((column) => <td key={column}>{String(row[column] ?? "")}</td>)}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function groupRows(rows: Record<string, unknown>[], fields: string[]) {
  const parentId = fields.find((field) => field === "id" || field.endsWith("_id")) ?? fields[0] ?? "row";
  const groups = new Map<string, { key: string; parentTitle: string; parent: Record<string, unknown>; children: Record<string, Record<string, unknown>[]> }>();

  for (const row of rows) {
    const key = String(row[parentId] ?? groups.size + 1);
    if (!groups.has(key)) {
      const parent = Object.fromEntries(fields.filter((field) => !field.includes("__")).map((field) => [field, row[field]]));
      groups.set(key, {
        key,
        parentTitle: `${parentId}: ${key}`,
        parent,
        children: {},
      });
    }
    const group = groups.get(key)!;
    const childColumns = fields.filter((field) => field.includes("__"));
    const prefixes = [...new Set(childColumns.map((field) => field.split("__")[0]))];
    for (const prefix of prefixes) {
      const child = Object.fromEntries(
        childColumns
          .filter((field) => field.startsWith(`${prefix}__`))
          .map((field) => [field.replace(`${prefix}__`, ""), row[field]]),
      );
      if (Object.values(child).some((value) => value !== null && value !== undefined)) {
        group.children[prefix] = group.children[prefix] ?? [];
        const childKey = JSON.stringify(child);
        if (!group.children[prefix].some((existing) => JSON.stringify(existing) === childKey)) {
          group.children[prefix].push(child);
        }
      }
    }
  }

  return [...groups.values()];
}

function ConnectionModal({ onClose, onSave }: { onClose(): void; onSave(connection: ConnectionInput): void }) {
  const [connection, setConnection] = useState<ConnectionInput>({
    name: "Local Postgres",
    type: "postgres",
    host: "127.0.0.1",
    port: 55432,
    database: "sqlg",
    user: "sqlg",
    password: "sqlg",
  });

  function update<K extends keyof ConnectionInput>(key: K, value: ConnectionInput[K]) {
    setConnection((current) => ({ ...current, [key]: value }));
  }

  return (
    <Modal title="Add Connection" onClose={onClose}>
      <form
        className="form-grid"
        onSubmit={(event) => {
          event.preventDefault();
          onSave(connection);
        }}
      >
        <label>Name<input value={connection.name} onChange={(event) => update("name", event.target.value)} /></label>
        <label>Type<select value={connection.type} onChange={(event) => {
          const type = event.target.value as ConnectionInput["type"];
          setConnection((current) => ({ ...current, type, name: type === "mysql" ? "Local MySQL" : "Local Postgres", port: type === "mysql" ? 33306 : 55432 }));
        }}><option value="postgres">Postgres</option><option value="mysql">MySQL</option></select></label>
        <label>Host<input value={connection.host} onChange={(event) => update("host", event.target.value)} /></label>
        <label>Port<input type="number" value={connection.port} onChange={(event) => update("port", Number(event.target.value))} /></label>
        <label>Database<input value={connection.database} onChange={(event) => update("database", event.target.value)} /></label>
        <label>User<input value={connection.user} onChange={(event) => update("user", event.target.value)} /></label>
        <label className="span-2">Password<input type="password" value={connection.password} onChange={(event) => update("password", event.target.value)} /></label>
        <div className="modal-actions"><button type="button" onClick={onClose}>Cancel</button><button className="primary" type="submit">Save</button></div>
      </form>
    </Modal>
  );
}

function SettingsModal({ settings, onClose, onSave }: { settings: Settings; onClose(): void; onSave(settings: Settings): void }) {
  const [draft, setDraft] = useState(settings);
  return (
    <Modal title="Settings" onClose={onClose}>
      <form className="form-grid" onSubmit={(event) => void (event.preventDefault(), onSave(draft))}>
        <label className="span-2">Theme<select value={draft.theme} onChange={(event) => setDraft({ theme: event.target.value as ThemeName })}>{themeOptions.map((theme) => <option value={theme.value} key={theme.value}>{theme.label}</option>)}</select></label>
        <div className="modal-actions"><button type="button" onClick={onClose}>Cancel</button><button className="primary" type="submit">Save</button></div>
      </form>
    </Modal>
  );
}

function Modal({ title, children, onClose }: { title: string; children: React.ReactNode; onClose(): void }) {
  return (
    <div className="modal-backdrop">
      <section className="modal">
        <header><h1>{title}</h1><button title="Close" className="icon-button" onClick={onClose}><X size={18} /></button></header>
        {children}
      </section>
    </div>
  );
}

export default App;
