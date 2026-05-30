import { useEffect, useMemo, useRef, useState } from "react";
import { basicSetup, EditorView } from "codemirror";
import { sql } from "@codemirror/lang-sql";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import { Database, Play, Plus, Settings as SettingsIcon, Trash2, X } from "lucide-react";
import type { Connection, ConnectionInput, QueryResult, ResultColumn, SchemaForeignKey, SchemaMetadata, Settings, TabName, ThemeName } from "./types";

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
    async executeQuery(payload) {
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
  const tree = useMemo(() => (result ? buildResultTree(result) : []), [result]);
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
        {tree.map((node) => <ResultNodeView node={node} key={node.id} />)}
      </div>
    </div>
  );
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
  const childUnique = schema.uniqueKeys.some(
    (key) => tableKey(key.schema, key.table) === childKey && key.columns.every((column) => foreignKey.fromColumns.includes(column)),
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
