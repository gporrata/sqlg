import { useEffect, useMemo, useRef, useState } from "react";
import { basicSetup, EditorView } from "codemirror";
import { sql } from "@codemirror/lang-sql";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import { Database, Pin, PinOff, Play, Plus, Settings as SettingsIcon, Trash2, X } from "lucide-react";
import type { Connection, ConnectionInput, ConnectionSchemaMetadata, QueryHistoryEntry, QueryResult, ResultColumn, ResultTabName, SchemaForeignKey, SchemaMetadata, SchemaTableMetadata, Settings, TabName, ThemeName } from "./types";

const themeOptions: { value: ThemeName; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "solarized", label: "Solarized" },
  { value: "gruvbox", label: "Gruvbox" },
  { value: "winter-solarized", label: "Winter Solarized" },
  { value: "ayu-light", label: "Ayu Light" },
];

const api = window.sqlg ?? createBrowserPreviewApi();

type QueryRunState = {
  id: string;
  sql: string;
  canceling: boolean;
};

type ResultTab = {
  id: string;
  label: string;
  result: QueryResult;
  pinned: boolean;
};

function App() {
  const [tab, setTab] = useState<TabName>("connections");
  const [connections, setConnections] = useState<Connection[]>([]);
  const [activeConnectionId, setActiveConnectionId] = useState("");
  const [sqlText, setSqlText] = useState('');
  const [queryHistory, setQueryHistory] = useState<QueryHistoryEntry[]>([]);
  const [resultTabs, setResultTabs] = useState<ResultTab[]>([]);
  const [footer, setFooter] = useState("Ready");
  const [settings, setSettings] = useState<Settings>({ theme: "system" });
  const [showConnectionModal, setShowConnectionModal] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [activeQuery, setActiveQuery] = useState<QueryRunState | null>(null);
  const [connectionSchemas, setConnectionSchemas] = useState<Record<string, ConnectionSchemaMetadata>>({});
  const [schemaStatus, setSchemaStatus] = useState<Record<string, { loading: boolean; error?: string }>>({});
  const editorRef = useRef<EditorView | null>(null);
  const activeQueryRef = useRef<QueryRunState | null>(null);
  const nextResultIndexRef = useRef(1);

  const activeConnection = connections.find((connection) => connection.id === activeConnectionId);
  const activeResultTabId = resultTabId(tab);
  const activeResultTab = activeResultTabId ? resultTabs.find((resultTab) => resultTab.id === activeResultTabId) ?? null : null;
  const isQueryRunning = Boolean(activeQuery);

  useEffect(() => {
    activeQueryRef.current = activeQuery;
  }, [activeQuery]);

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

  useEffect(() => {
    if (!activeConnectionId) {
      setQueryHistory([]);
      return;
    }
    void refreshQueryHistory(activeConnectionId);
    void loadConnectionSchema(activeConnectionId);
  }, [activeConnectionId]);

  async function refreshConnections() {
    const items = await api.listConnections();
    setConnections(items);
    setActiveConnectionId((current) => (items.some((item) => item.id === current) ? current : items[0]?.id || ""));
  }

  async function refreshQueryHistory(connectionId = activeConnectionId) {
    if (!connectionId) {
      setQueryHistory([]);
      return;
    }
    try {
      setQueryHistory(await api.listQueryHistory(connectionId));
    } catch (error) {
      setFooter(error instanceof Error ? error.message : String(error));
    }
  }

  async function loadConnectionSchema(connectionId = activeConnectionId, force = false) {
    if (!connectionId || (!force && connectionSchemas[connectionId])) {
      return;
    }
    setSchemaStatus((current) => ({ ...current, [connectionId]: { loading: true } }));
    try {
      const schema = await api.loadConnectionSchema(connectionId, force);
      setConnectionSchemas((current) => ({ ...current, [connectionId]: schema }));
      setSchemaStatus((current) => ({ ...current, [connectionId]: { loading: false } }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setSchemaStatus((current) => ({ ...current, [connectionId]: { loading: false, error: message } }));
      setFooter(message);
    }
  }

  async function executeCurrentSql() {
    if (activeQueryRef.current) {
      return;
    }
    if (!activeConnectionId) {
      setFooter("Select a connection before executing SQL");
      return;
    }
    const editor = editorRef.current;
    const state = editor?.state;
    const selection = state ? state.sliceDoc(state.selection.main.from, state.selection.main.to).trim() : "";
    const query = selection || editor?.state.doc.toString() || sqlText;
    const queryId = `${activeConnectionId}-${Date.now()}`;
    const nextActiveQuery = { id: queryId, sql: query, canceling: false };
    activeQueryRef.current = nextActiveQuery;
    setActiveQuery(nextActiveQuery);
    try {
      setFooter("Executing query...");
      const nextResult = await api.executeQuery({ connectionId: activeConnectionId, sql: query, queryId });
      const currentQuery = activeQueryRef.current;
      if (currentQuery?.id !== queryId) {
        return;
      }
      if (currentQuery.canceling) {
        setFooter("Query canceled");
        await refreshQueryHistory(activeConnectionId);
        return;
      }
      const nextResultTab: ResultTab = {
        id: queryId,
        label: `Result ${nextResultIndexRef.current}`,
        result: nextResult,
        pinned: false,
      };
      nextResultIndexRef.current += 1;
      setResultTabs((current) => [...current.filter((resultTab) => resultTab.pinned), nextResultTab]);
      setTab(resultTabName(nextResultTab.id));
      setFooter(`${nextResult.rowCount} row(s) returned in ${nextResult.elapsedMs}ms`);
      await refreshQueryHistory(activeConnectionId);
    } catch (error) {
      if (activeQueryRef.current?.id === queryId) {
        setFooter(activeQueryRef.current.canceling ? "Query canceled" : error instanceof Error ? error.message : String(error));
      }
      await refreshQueryHistory(activeConnectionId);
    } finally {
      if (activeQueryRef.current?.id === queryId) {
        activeQueryRef.current = null;
        setActiveQuery(null);
      }
    }
  }

  async function cancelActiveQuery() {
    const query = activeQueryRef.current;
    if (!query || query.canceling) {
      return;
    }
    const cancelingQuery = { ...query, canceling: true };
    activeQueryRef.current = cancelingQuery;
    setActiveQuery(cancelingQuery);
    setFooter("Canceling query...");
    try {
      const canceled = await api.cancelQuery(query.id);
      setFooter(canceled ? "Query cancellation requested" : "Query is already finishing");
    } catch (error) {
      setFooter(error instanceof Error ? error.message : String(error));
      setActiveQuery((current) => (current?.id === query.id ? { ...current, canceling: false } : current));
    }
  }

  function copyHistorySqlToEditor(sql: string) {
    setSqlText(sql);
    setTab("editor");
  }

  function toggleResultTabPinned(id: string) {
    setResultTabs((current) => current.map((resultTab) => (resultTab.id === id ? { ...resultTab, pinned: !resultTab.pinned } : resultTab)));
  }

  return (
    <main className="app-shell">
      <header className="app-header">
        <div className="brand">
          <Database size={20} />
          <span>sqlg</span>
        </div>
        <nav className="tabs" aria-label="Main tabs">
          <button className={tab === "connections" ? "active" : ""} disabled={isQueryRunning} onClick={() => setTab("connections")}>
            Connections
          </button>
          <button className={tab === "editor" ? "active" : ""} disabled={isQueryRunning} onClick={() => setTab("editor")}>
            SQL
          </button>
          {resultTabs.map((resultTab) => {
            const tabName = resultTabName(resultTab.id);
            return (
              <button
                className={`${tab === tabName ? "active " : ""}result-tab-button`}
                disabled={isQueryRunning}
                key={resultTab.id}
                title={`${resultTab.label}: ${resultTab.result.rowCount} row(s), ${resultTab.result.elapsedMs}ms`}
                onClick={() => setTab(tabName)}
              >
                <span>{resultTab.label}</span>
                {resultTab.pinned && <Pin size={13} aria-hidden="true" />}
              </button>
            );
          })}
          <button className={tab === "history" ? "active" : ""} disabled={isQueryRunning} onClick={() => setTab("history")}>
            History
          </button>
        </nav>
      </header>

      <section className="workspace" aria-busy={isQueryRunning}>
        {tab === "connections" && (
          <ConnectionsView
            connections={connections}
            activeConnectionId={activeConnectionId}
            disabled={isQueryRunning}
            schemas={connectionSchemas}
            schemaStatus={schemaStatus}
            onSelect={setActiveConnectionId}
            onRefreshSchema={(id) => loadConnectionSchema(id, true)}
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
            isExecuting={isQueryRunning}
            onChange={setSqlText}
            onExecute={executeCurrentSql}
            editorRef={editorRef}
          />
        )}
        {activeResultTabId && <ResultsView resultTab={activeResultTab} onTogglePinned={toggleResultTabPinned} />}
        {tab === "history" && <HistoryView activeConnection={activeConnection} history={queryHistory} onSelectQuery={copyHistorySqlToEditor} />}
      </section>

      <footer className="footer">
        <span>{footer}</span>
        <button title="Settings" className="icon-button" disabled={isQueryRunning} onClick={() => setShowSettingsModal(true)}>
          <SettingsIcon size={18} />
        </button>
      </footer>

      {activeQuery && <QueryOverlay query={activeQuery} onCancel={cancelActiveQuery} />}

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
          onChange={async (next) => {
            const saved = await api.saveSettings(next);
            setSettings(saved);
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
    async loadConnectionSchema() {
      return {
        provider: "postgres",
        database: "sqlg",
        schemas: [
          {
            name: "public",
            tables: [
              {
                schema: "public",
                name: "customers",
                tableType: "table",
                characteristics: ["persistence: permanent", "estimated rows: 2"],
                columns: [
                  { name: "id", dataType: "integer", nullable: false, isPrimaryKey: true },
                  { name: "name", dataType: "text", nullable: false },
                  { name: "email", dataType: "text", nullable: false, isUnique: true },
                ],
              },
              {
                schema: "public",
                name: "orders",
                tableType: "table",
                characteristics: ["persistence: permanent", "estimated rows: 3"],
                columns: [
                  { name: "id", dataType: "integer", nullable: false, isPrimaryKey: true },
                  { name: "customer_id", dataType: "integer", nullable: false, foreignKey: "public.customers.id" },
                  { name: "total", dataType: "numeric(10,2)", nullable: false },
                ],
              },
            ],
          },
        ],
      };
    },
    async executeQuery(payload) {
      const startedAt = Date.now();
      const executedAt = new Date().toISOString();
      const elapsedMs = 18;
      const historyEntry: QueryHistoryEntry = {
        id: `preview-${Date.now()}`,
        connectionId: payload.connectionId,
        connectionName: fallbackConnection.name,
        sql: payload.sql,
        rowCount: 3,
        elapsedMs,
        executedAt,
        status: "success",
      };
      const currentHistory = await this.listQueryHistory(payload.connectionId);
      localStorage.setItem(`sqlg-preview-history-${payload.connectionId}`, JSON.stringify([historyEntry, ...currentHistory]));
      return {
        rows: [
          { id: 1, name: "Ada Lovelace", email: "ada@example.com", orders__id: 1, orders__order_number: "PG-1001", orders__total: "199.50", items__id: 1, items__sku: "SQL-BOOK", items__quantity: 1 },
          { id: 1, name: "Ada Lovelace", email: "ada@example.com", orders__id: 1, orders__order_number: "PG-1001", orders__total: "199.50", items__id: 2, items__sku: "QUERY-MUG", items__quantity: 2 },
          { id: 2, name: "Grace Hopper", email: "grace@example.com", orders__id: 3, orders__order_number: "PG-1003", orders__total: "75.25", items__id: 4, items__sku: "PLAN-HOODIE", items__quantity: 1 },
        ],
        fields: ["id", "name", "email", "orders__id", "orders__order_number", "orders__total", "items__id", "items__sku", "items__quantity"],
        columns: [
          { name: "id", schema: "public", table: "customers", column: "id" },
          { name: "name", schema: "public", table: "customers", column: "name" },
          { name: "email", schema: "public", table: "customers", column: "email" },
          { name: "orders__id", schema: "public", table: "orders", column: "id" },
          { name: "orders__order_number", schema: "public", table: "orders", column: "order_number" },
          { name: "orders__total", schema: "public", table: "orders", column: "total" },
          { name: "items__id", schema: "public", table: "order_items", column: "id" },
          { name: "items__sku", schema: "public", table: "order_items", column: "sku" },
          { name: "items__quantity", schema: "public", table: "order_items", column: "quantity" },
        ],
        schema: {
          foreignKeys: [
            { fromSchema: "public", fromTable: "orders", fromColumns: ["customer_id"], toSchema: "public", toTable: "customers", toColumns: ["id"] },
            { fromSchema: "public", fromTable: "order_items", fromColumns: ["order_id"], toSchema: "public", toTable: "orders", toColumns: ["id"] },
          ],
          uniqueKeys: [
            { schema: "public", table: "customers", columns: ["id"] },
            { schema: "public", table: "orders", columns: ["id"] },
            { schema: "public", table: "order_items", columns: ["id"] },
          ],
        },
        rowCount: 3,
        connectionName: fallbackConnection.name,
        sql: payload.sql,
        elapsedMs: Math.max(elapsedMs, Date.now() - startedAt),
        executedAt,
      };
    },
    async cancelQuery() {
      return false;
    },
    async listQueryHistory(connectionId) {
      const stored = localStorage.getItem(`sqlg-preview-history-${connectionId}`);
      return stored ? JSON.parse(stored) : [];
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
  disabled: boolean;
  schemas: Record<string, ConnectionSchemaMetadata>;
  schemaStatus: Record<string, { loading: boolean; error?: string }>;
  onSelect(id: string): void;
  onRefreshSchema(id: string): void;
  onAdd(): void;
  onDelete(id: string): void;
  onTest(id: string): void;
}) {
  return (
    <div className="pane">
      <div className="pane-toolbar">
        <h1>Connections</h1>
        <button className="primary" disabled={props.disabled} onClick={props.onAdd}>
          <Plus size={16} />
          Add Connection
        </button>
      </div>
      <div className="connection-list">
        {props.connections.map((connection) => (
          <article
            className={`connection-row ${connection.id === props.activeConnectionId ? "selected" : ""}`}
            key={connection.id}
            onClick={() => {
              if (!props.disabled) props.onSelect(connection.id);
            }}
          >
            <div>
              <strong>{connection.name}</strong>
              <span>
                {connection.type}://{connection.user}@{connection.host}:{connection.port}/{connection.database}
              </span>
            </div>
            <div className="row-actions">
              <button disabled={props.disabled} onClick={(event) => void (event.stopPropagation(), props.onTest(connection.id))}>Test</button>
              {connection.id === props.activeConnectionId && (
                <button disabled={props.disabled || props.schemaStatus[connection.id]?.loading} onClick={(event) => void (event.stopPropagation(), props.onRefreshSchema(connection.id))}>Refresh Schema</button>
              )}
              <button className="icon-button danger" title="Delete" disabled={props.disabled} onClick={(event) => void (event.stopPropagation(), props.onDelete(connection.id))}>
                <Trash2 size={16} />
              </button>
            </div>
            {connection.id === props.activeConnectionId && (
              <ConnectionSchemaTree
                schema={props.schemas[connection.id]}
                status={props.schemaStatus[connection.id]}
              />
            )}
          </article>
        ))}
        {props.connections.length === 0 && <p className="empty">No connections yet. Add one for the Docker sample databases.</p>}
      </div>
    </div>
  );
}

function ConnectionSchemaTree({ schema, status }: { schema?: ConnectionSchemaMetadata; status?: { loading: boolean; error?: string } }) {
  if (status?.loading) {
    return <div className="schema-tree schema-state">Loading schema...</div>;
  }
  if (status?.error) {
    return <div className="schema-tree schema-state error">{status.error}</div>;
  }
  if (!schema) {
    return <div className="schema-tree schema-state">Select the connection to load schema metadata.</div>;
  }

  return (
    <div className="schema-tree">
      <details open>
        <summary>
          <span>{schema.database}</span>
          <small>{schema.provider}</small>
        </summary>
        <div className="schema-children">
          {schema.schemas.map((schemaNode) => (
            <details open key={schemaNode.name}>
              <summary>
                <span>{schemaNode.name}</span>
                <small>
                  {schemaNode.tables.length} table{schemaNode.tables.length === 1 ? "" : "s"}
                </small>
              </summary>
              <div className="schema-children">
                {schemaNode.tables.map((table) => <SchemaTableTree table={table} key={`${table.schema}.${table.name}`} />)}
              </div>
            </details>
          ))}
        </div>
      </details>
    </div>
  );
}

function SchemaTableTree({ table }: { table: SchemaTableMetadata }) {
  return (
    <details>
      <summary>
        <span>{table.name}</span>
        <small>{table.tableType}</small>
      </summary>
      <div className="schema-children">
        {table.characteristics.length > 0 && (
          <div className="schema-characteristics">
            {table.characteristics.map((characteristic) => <span key={characteristic}>{characteristic}</span>)}
          </div>
        )}
        <ul className="schema-columns">
          {table.columns.map((column) => (
            <li key={column.name}>
              <div>
                <strong>{column.name}</strong>
                <code>{column.dataType}</code>
              </div>
              <span>{column.nullable ? "nullable" : "not null"}</span>
              {column.isPrimaryKey && <span>primary key</span>}
              {column.isUnique && !column.isPrimaryKey && <span>unique</span>}
              {column.foreignKey && <span>references {column.foreignKey}</span>}
              {column.defaultValue && <span>default {column.defaultValue}</span>}
              {column.extra?.map((item) => <span key={item}>{item}</span>)}
            </li>
          ))}
        </ul>
      </div>
    </details>
  );
}

function EditorViewPane(props: {
  activeConnection?: Connection;
  sqlText: string;
  settings: Settings;
  isExecuting: boolean;
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
          props.settings.theme === "dark" || props.settings.theme === "solarized" || props.settings.theme === "gruvbox" ? oneDark : [],
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

  useEffect(() => {
    const view = props.editorRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current === props.sqlText) return;
    view.dispatch({
      changes: { from: 0, to: current.length, insert: props.sqlText },
    });
  }, [props.sqlText]);

  return (
    <div className="pane editor-pane">
      <div className="pane-toolbar">
        <div>
          <h1>SQL Editor</h1>
          <p>{props.activeConnection ? `Using ${props.activeConnection.name}` : "No connection selected"}</p>
        </div>
        <button className="primary" disabled={!props.activeConnection || props.isExecuting} onClick={props.onExecute}>
          <Play size={16} />
          {props.isExecuting ? "Running" : "Execute"}
        </button>
      </div>
      <div className="editor-host" ref={hostRef} />
    </div>
  );
}

function ResultsView({ resultTab, onTogglePinned }: { resultTab: ResultTab | null; onTogglePinned(id: string): void }) {
  const result = resultTab?.result ?? null;
  const tree = useMemo(() => (result ? buildResultTree(result) : []), [result]);
  if (!resultTab || !result) {
    return <div className="pane empty">Execute a query to see hierarchical results.</div>;
  }
  return (
    <div className="pane results-pane">
      <div className="pane-toolbar">
        <div>
          <h1>{resultTab.label}</h1>
          <p>
            {result.connectionName} | {result.rowCount} row(s) | {result.elapsedMs}ms
          </p>
        </div>
        <div className="toolbar-actions">
          <button
            className={`icon-button pin-toggle ${resultTab.pinned ? "active" : ""}`}
            title={resultTab.pinned ? "Unpin result tab" : "Pin result tab"}
            aria-label={resultTab.pinned ? "Unpin result tab" : "Pin result tab"}
            aria-pressed={resultTab.pinned}
            onClick={() => onTogglePinned(resultTab.id)}
          >
            {resultTab.pinned ? <PinOff size={16} /> : <Pin size={16} />}
          </button>
        </div>
      </div>
      <div className="result-tree">
        {tree.map((node) => <ResultNodeView node={node} key={node.id} />)}
      </div>
    </div>
  );
}

function HistoryView({
  activeConnection,
  history,
  onSelectQuery,
}: {
  activeConnection?: Connection;
  history: QueryHistoryEntry[];
  onSelectQuery(sql: string): void;
}) {
  if (!activeConnection) {
    return <div className="pane empty">Select a connection to see query history.</div>;
  }
  return (
    <div className="pane history-pane">
      <div className="pane-toolbar">
        <div>
          <h1>History</h1>
          <p>
            {activeConnection.name} | {history.length} quer{history.length === 1 ? "y" : "ies"}
          </p>
        </div>
      </div>
      <div className="history-list">
        {history.map((entry) => (
          <article className="history-row" key={entry.id} role="button" tabIndex={0} onClick={() => onSelectQuery(entry.sql)} onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              onSelectQuery(entry.sql);
            }
          }}>
            <div className="history-meta">
              <span className={`history-status ${entry.status}`}>{entry.status}</span>
              <span>{formatDateTime(entry.executedAt)}</span>
              <span>{entry.elapsedMs}ms</span>
              {entry.status === "success" && <span>{entry.rowCount ?? 0} row(s)</span>}
            </div>
            <pre>{entry.sql}</pre>
            {entry.error && <p className="history-error">{entry.error}</p>}
          </article>
        ))}
        {history.length === 0 && <p className="empty">No queries have been executed for this connection yet.</p>}
      </div>
    </div>
  );
}

function QueryOverlay({ query, onCancel }: { query: QueryRunState; onCancel(): void }) {
  return (
    <div className="query-overlay" role="status" aria-live="polite">
      <div className="query-loader" aria-hidden="true">
        <Database size={42} />
        <span />
      </div>
      <div>
        <h2>Running query</h2>
        <p>{query.canceling ? "Canceling..." : "Waiting for the database to return results."}</p>
      </div>
      <pre>{query.sql}</pre>
      <button className="danger" disabled={query.canceling} onClick={onCancel}>
        <X size={16} />
        {query.canceling ? "Canceling" : "Cancel Query"}
      </button>
    </div>
  );
}

function formatDateTime(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(new Date(value));
}

function resultTabName(id: string): ResultTabName {
  return `result:${id}`;
}

function resultTabId(tabName: TabName) {
  return tabName.startsWith("result:") ? tabName.slice("result:".length) : null;
}

function ResultNodeView({ node }: { node: ResultNode }) {
  return (
    <details className={`result-node ${node.cardinality}`} open>
      <summary>
        <span>{node.label}</span>
        <small>{node.title}</small>
      </summary>
      <DataTable rows={[node.row]} />
      {node.children.length > 0 && (
        <div className="result-children">
          {node.children.map((child) => <ResultNodeView node={child} key={child.id} />)}
        </div>
      )}
    </details>
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

type ResultNode = {
  id: string;
  label: string;
  title: string;
  tableKey: string;
  rowKey: string;
  row: Record<string, unknown>;
  cardinality: "one" | "many";
  children: ResultNode[];
  childIds: Set<string>;
};

type TableSelection = {
  key: string;
  schema: string;
  table: string;
  columns: ResultColumn[];
};

type TableRow = TableSelection & {
  row: Record<string, unknown>;
  rowKey: string;
};

type Relationship = {
  parentKey: string;
  childKey: string;
  cardinality: "one" | "many";
};

function buildResultTree(result: QueryResult) {
  const tables = selectedTables(result.columns);
  if (!tables.length) {
    return result.rows.map((row, index) => createNode("row", "row", String(index + 1), row, "one"));
  }

  const relationships = relationshipsFor(tables, result.schema);
  const rootKeys = rootTableKeys(tables, relationships);
  const nodes = new Map<string, ResultNode>();
  const roots: ResultNode[] = [];

  for (let rowIndex = 0; rowIndex < result.rows.length; rowIndex += 1) {
    const tableRows = tableRowsFor(result.rows[rowIndex], tables);
    if (!tableRows.length) {
      const node = createNode("row", "row", String(rowIndex + 1), result.rows[rowIndex], "one");
      roots.push(node);
      continue;
    }

    const rowRootKeys = rootKeys.filter((key) => tableRows.some((tableRow) => tableRow.key === key));
    const primaryRoot = rowRootKeys[0] ?? tableRows[0].key;
    for (const rootKey of rowRootKeys.length ? rowRootKeys : [primaryRoot]) {
      const rootRow = tableRows.find((tableRow) => tableRow.key === rootKey);
      if (!rootRow) continue;
      const root = getOrCreateNode(rootRow, nodes, "one");
      if (!roots.some((node) => node.id === root.id)) roots.push(root);
      attachKnownChildren(root, tableRows, relationships, nodes);
      attachUnknownChildren(root, tableRows, relationships, nodes);
    }
  }

  return roots;
}

function selectedTables(columns: ResultColumn[]): TableSelection[] {
  const tables = new Map<string, TableSelection>();
  for (const column of columns) {
    const schema = column.schema ?? "__result";
    const tableName = column.table ?? "row";
    const key = tableKey(schema, tableName);
    const table = tables.get(key) ?? { key, schema, table: tableName, columns: [] };
    table.columns.push(column);
    tables.set(key, table);
  }
  return [...tables.values()];
}

function relationshipsFor(tables: TableSelection[], schema: SchemaMetadata) {
  const selected = new Set(tables.map((table) => table.key));
  return schema.foreignKeys
    .map((foreignKey) => relationshipFor(foreignKey, schema))
    .filter((relationship) => selected.has(relationship.parentKey) && selected.has(relationship.childKey));
}

function relationshipFor(foreignKey: SchemaForeignKey, schema: SchemaMetadata): Relationship {
  const childKey = tableKey(foreignKey.fromSchema, foreignKey.fromTable);
  const parentKey = tableKey(foreignKey.toSchema, foreignKey.toTable);
  const fromColumns = listValue(foreignKey.fromColumns);
  const childUnique = schema.uniqueKeys.some(
    (key) => tableKey(key.schema, key.table) === childKey && listValue(key.columns).every((column) => fromColumns.includes(column)),
  );
  return { parentKey, childKey, cardinality: childUnique ? "one" : "many" };
}

function rootTableKeys(tables: TableSelection[], relationships: Relationship[]) {
  const childKeys = new Set(relationships.map((relationship) => relationship.childKey));
  const roots = tables.map((table) => table.key).filter((key) => !childKeys.has(key));
  return roots.length ? roots : [tables[0].key];
}

function tableRowsFor(row: Record<string, unknown>, tables: TableSelection[]) {
  return tables.flatMap((table) => {
    const tableRow = Object.fromEntries(table.columns.map((column) => [column.name, row[column.name]]));
    if (!Object.values(tableRow).some((value) => value !== null && value !== undefined)) return [];
    return [{ ...table, row: tableRow, rowKey: stableRowKey(tableRow) }];
  });
}

function attachKnownChildren(parent: ResultNode, tableRows: TableRow[], relationships: Relationship[], nodes: Map<string, ResultNode>) {
  const childRelationships = relationships.filter((relationship) => relationship.parentKey === parent.tableKey);
  for (const relationship of childRelationships) {
    const childRow = tableRows.find((tableRow) => tableRow.key === relationship.childKey);
    if (!childRow) continue;
    const child = getOrCreateNode(childRow, nodes, relationship.cardinality);
    attachChild(parent, child);
    attachKnownChildren(child, tableRows, relationships, nodes);
  }
}

function attachUnknownChildren(parent: ResultNode, tableRows: TableRow[], relationships: Relationship[], nodes: Map<string, ResultNode>) {
  const related = new Set(relationships.flatMap((relationship) => [relationship.parentKey, relationship.childKey]));
  for (const tableRow of tableRows) {
    if (tableRow.key === parent.tableKey || related.has(tableRow.key)) continue;
    attachChild(parent, getOrCreateNode(tableRow, nodes, "one"));
  }
}

function getOrCreateNode(tableRow: TableRow, nodes: Map<string, ResultNode>, cardinality: "one" | "many") {
  const id = `${tableRow.key}:${tableRow.rowKey}`;
  const existing = nodes.get(id);
  if (existing) {
    if (cardinality === "many") existing.cardinality = "many";
    return existing;
  }
  const node = createNode(id, tableRow.key, tableRow.table, tableRow.row, cardinality);
  nodes.set(id, node);
  return node;
}

function createNode(id: string, tableKey: string, label: string, row: Record<string, unknown>, cardinality: "one" | "many"): ResultNode {
  return {
    id,
    label,
    title: [...new Set(Object.values(row))].map(String).join(" | ") || label,
    tableKey,
    rowKey: stableRowKey(row),
    row,
    cardinality,
    children: [],
    childIds: new Set(),
  };
}

function attachChild(parent: ResultNode, child: ResultNode) {
  if (parent.childIds.has(child.id) || parent.id === child.id) return;
  parent.childIds.add(child.id);
  parent.children.push(child);
}

function stableRowKey(row: Record<string, unknown>) {
  return JSON.stringify(Object.keys(row).sort().map((key) => [key, row[key]]));
}

function tableKey(schema: string, table: string) {
  return `${schema}.${table}`;
}

function listValue<T>(value: T[] | T | null | undefined): T[] {
  if (Array.isArray(value)) return value;
  if (value === null || value === undefined) return [];
  return [value];
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

function SettingsModal({ settings, onClose, onChange }: { settings: Settings; onClose(): void; onChange(settings: Settings): void }) {
  return (
    <Modal title="Settings" onClose={onClose}>
      <div className="form-grid">
        <label className="span-2">Theme<select value={settings.theme} onChange={(event) => onChange({ theme: event.target.value as ThemeName })}>{themeOptions.map((theme) => <option value={theme.value} key={theme.value}>{theme.label}</option>)}</select></label>
      </div>
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
