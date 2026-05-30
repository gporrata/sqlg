const { app, BrowserWindow, Menu, ipcMain } = require("electron");
const path = require("node:path");
const fs = require("node:fs");
const mysql = require("mysql2/promise");
const { Client } = require("pg");

const isDev = process.env.NODE_ENV !== "production";
const schemaCache = new Map();

app.setName("sqlg");

function storePath() {
  return path.join(app.getPath("home"), ".config", "sql", "config.json");
}

function emptyStore() {
  return {
    connections: [],
    settings: { theme: "system" },
  };
}

function readStore() {
  try {
    const raw = fs.readFileSync(storePath(), "utf8");
    return { ...emptyStore(), ...JSON.parse(raw) };
  } catch {
    return emptyStore();
  }
}

function writeStore(store) {
  fs.mkdirSync(path.dirname(storePath()), { recursive: true });
  fs.writeFileSync(storePath(), JSON.stringify(store, null, 2));
}

function publicConnection(connection) {
  const { password, ...safe } = connection;
  return { ...safe, hasPassword: Boolean(password) };
}

function findConnection(id) {
  const connection = readStore().connections.find((item) => item.id === id);
  if (!connection) {
    throw new Error("Connection not found");
  }
  return connection;
}

function tableKey(schema, table) {
  return `${schema}.${table}`;
}

function normalizeConnection(input) {
  const type = input.type === "mysql" ? "mysql" : "postgres";
  const defaultPort = type === "mysql" ? 33306 : 55432;
  return {
    id: input.id || `${type}-${Date.now()}`,
    name: String(input.name || `${type} local`).trim(),
    type,
    host: String(input.host || "127.0.0.1").trim(),
    port: Number(input.port || defaultPort),
    database: String(input.database || "sqlg").trim(),
    user: String(input.user || "sqlg").trim(),
    password: String(input.password || ""),
  };
}

async function withClient(connection, run) {
  if (connection.type === "mysql") {
    const client = await mysql.createConnection({
      host: connection.host,
      port: connection.port,
      database: connection.database,
      user: connection.user,
      password: connection.password,
      multipleStatements: false,
    });
    try {
      return await run(client);
    } finally {
      await client.end();
    }
  }

  const client = new Client({
    host: connection.host,
    port: connection.port,
    database: connection.database,
    user: connection.user,
    password: connection.password,
  });
  await client.connect();
  try {
    return await run(client);
  } finally {
    await client.end();
  }
}

async function loadPostgresSchema(client) {
  const foreignKeyResult = await client.query(`
    select
      nsp.nspname as from_schema,
      cls.relname as from_table,
      array_agg(att.attname order by fk.ord) as from_columns,
      fnsp.nspname as to_schema,
      fcls.relname as to_table,
      array_agg(fatt.attname order by fk.ord) as to_columns
    from pg_constraint con
    join pg_class cls on cls.oid = con.conrelid
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    join pg_class fcls on fcls.oid = con.confrelid
    join pg_namespace fnsp on fnsp.oid = fcls.relnamespace
    join unnest(con.conkey) with ordinality as fk(attnum, ord) on true
    join pg_attribute att on att.attrelid = con.conrelid and att.attnum = fk.attnum
    join unnest(con.confkey) with ordinality as pk(attnum, ord) on pk.ord = fk.ord
    join pg_attribute fatt on fatt.attrelid = con.confrelid and fatt.attnum = pk.attnum
    where con.contype = 'f'
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
      and fnsp.nspname not in ('pg_catalog', 'information_schema')
      and fnsp.nspname not like 'pg_toast%'
    group by con.oid, nsp.nspname, cls.relname, fnsp.nspname, fcls.relname
  `);
  const uniqueKeyResult = await client.query(`
    select
      nsp.nspname as schema,
      cls.relname as table,
      array_agg(att.attname order by key.ord) as columns
    from pg_constraint con
    join pg_class cls on cls.oid = con.conrelid
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    join unnest(con.conkey) with ordinality as key(attnum, ord) on true
    join pg_attribute att on att.attrelid = con.conrelid and att.attnum = key.attnum
    where con.contype in ('p', 'u')
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
    group by con.oid, nsp.nspname, cls.relname
  `);
  const columnResult = await client.query(`
    select
      cls.oid as table_id,
      att.attnum as column_id,
      nsp.nspname as schema,
      cls.relname as table,
      att.attname as column
    from pg_attribute att
    join pg_class cls on cls.oid = att.attrelid
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    where att.attnum > 0
      and not att.attisdropped
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
  `);
  const columnsById = Object.fromEntries(
    columnResult.rows.map((row) => [`${row.table_id}:${row.column_id}`, { schema: row.schema, table: row.table, column: row.column }]),
  );

  return {
    foreignKeys: foreignKeyResult.rows.map((row) => ({
      fromSchema: row.from_schema,
      fromTable: row.from_table,
      fromColumns: row.from_columns,
      toSchema: row.to_schema,
      toTable: row.to_table,
      toColumns: row.to_columns,
    })),
    uniqueKeys: uniqueKeyResult.rows.map((row) => ({ schema: row.schema, table: row.table, columns: row.columns })),
    columnsById,
  };
}

async function loadMysqlSchema(client) {
  const [foreignKeys] = await client.query(`
    select
      table_schema as from_schema,
      table_name as from_table,
      constraint_name,
      column_name as from_column,
      referenced_table_schema as to_schema,
      referenced_table_name as to_table,
      referenced_column_name as to_column,
      ordinal_position
    from information_schema.key_column_usage
    where table_schema = database()
      and referenced_table_name is not null
    order by table_schema, table_name, constraint_name, ordinal_position
  `);
  const [uniqueKeys] = await client.query(`
    select
      kcu.table_schema as \`schema\`,
      kcu.table_name as \`table\`,
      kcu.constraint_name,
      kcu.column_name as \`column\`,
      kcu.ordinal_position
    from information_schema.table_constraints tc
    join information_schema.key_column_usage kcu
      on kcu.constraint_schema = tc.constraint_schema
      and kcu.table_schema = tc.table_schema
      and kcu.table_name = tc.table_name
      and kcu.constraint_name = tc.constraint_name
    where tc.table_schema = database()
      and tc.constraint_type in ('PRIMARY KEY', 'UNIQUE')
    order by kcu.table_schema, kcu.table_name, kcu.constraint_name, kcu.ordinal_position
  `);
  const groupedForeignKeys = new Map();
  for (const row of foreignKeys) {
    const key = `${row.from_schema}.${row.from_table}.${row.constraint_name}`;
    const current = groupedForeignKeys.get(key) ?? {
      fromSchema: row.from_schema,
      fromTable: row.from_table,
      fromColumns: [],
      toSchema: row.to_schema,
      toTable: row.to_table,
      toColumns: [],
    };
    current.fromColumns.push(row.from_column);
    current.toColumns.push(row.to_column);
    groupedForeignKeys.set(key, current);
  }
  const groupedUniqueKeys = new Map();
  for (const row of uniqueKeys) {
    const key = `${row.schema}.${row.table}.${row.constraint_name}`;
    const current = groupedUniqueKeys.get(key) ?? { schema: row.schema, table: row.table, columns: [] };
    current.columns.push(row.column);
    groupedUniqueKeys.set(key, current);
  }

  return {
    foreignKeys: [...groupedForeignKeys.values()],
    uniqueKeys: [...groupedUniqueKeys.values()],
    columnsById: {},
  };
}

async function getSchema(connection, client) {
  if (schemaCache.has(connection.id)) {
    return schemaCache.get(connection.id);
  }
  const schema = connection.type === "mysql" ? await loadMysqlSchema(client) : await loadPostgresSchema(client);
  schemaCache.set(connection.id, schema);
  return schema;
}

function publicSchema(schema) {
  return {
    foreignKeys: schema.foreignKeys,
    uniqueKeys: schema.uniqueKeys,
  };
}

function postgresColumns(fields, schema) {
  return fields.map((field) => ({
    name: field.name,
    ...(schema.columnsById[`${field.tableID}:${field.columnID}`] ?? {}),
  }));
}

function mysqlColumns(fields) {
  return fields.map((field) => ({
    name: field.name,
    schema: field.db || undefined,
    table: field.orgTable || field.table || undefined,
    column: field.orgName || field.name || undefined,
  }));
}

async function executeSql(connection, sql) {
  if (!sql.trim()) {
    throw new Error("No SQL to execute");
  }

  return withClient(connection, async (client) => {
    const schema = await getSchema(connection, client);
    if (connection.type === "mysql") {
      const [rows, fields] = await client.query(sql);
      return {
        rows: Array.isArray(rows) ? rows : [],
        fields: fields.map((field) => field.name),
        columns: mysqlColumns(fields),
        schema: publicSchema(schema),
        rowCount: Array.isArray(rows) ? rows.length : 0,
      };
    }

    const result = await client.query(sql);
    return {
      rows: result.rows,
      fields: result.fields.map((field) => field.name),
      columns: postgresColumns(result.fields, schema),
      schema: publicSchema(schema),
      rowCount: result.rowCount ?? result.rows.length,
    };
  });
}

function createWindow() {
  const win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 920,
    minHeight: 640,
    title: "sqlg",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (isDev) {
    win.loadURL("http://localhost:5173");
  } else {
    win.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }
}

function createAppMenu() {
  if (process.platform !== "darwin") {
    return;
  }
  Menu.setApplicationMenu(
    Menu.buildFromTemplate([
      {
        label: "sqlg",
        submenu: [
          { role: "about", label: "About sqlg" },
          { type: "separator" },
          { role: "services" },
          { type: "separator" },
          { role: "hide", label: "Hide sqlg" },
          { role: "hideOthers" },
          { role: "unhide" },
          { type: "separator" },
          { role: "quit", label: "Quit sqlg" },
        ],
      },
      { role: "editMenu" },
      { role: "viewMenu" },
      { role: "windowMenu" },
    ]),
  );
}

ipcMain.handle("connections:list", () => {
  return readStore().connections.map(publicConnection);
});

ipcMain.handle("connections:add", (_event, input) => {
  const store = readStore();
  const connection = normalizeConnection(input);
  if (!connection.name || !connection.host || !connection.database || !connection.user) {
    throw new Error("Name, host, database, and user are required");
  }
  const existing = store.connections.filter((item) => item.id !== connection.id);
  store.connections = [...existing, connection];
  schemaCache.delete(connection.id);
  writeStore(store);
  return publicConnection(connection);
});

ipcMain.handle("connections:delete", (_event, id) => {
  const store = readStore();
  store.connections = store.connections.filter((item) => item.id !== id);
  schemaCache.delete(id);
  writeStore(store);
  return true;
});

ipcMain.handle("connections:test", async (_event, id) => {
  const connection = findConnection(id);
  await withClient(connection, async (client) => {
    if (connection.type === "mysql") {
      await client.query("select 1");
    } else {
      await client.query("select 1");
    }
  });
  return true;
});

ipcMain.handle("query:execute", async (_event, payload) => {
  const connection = findConnection(payload.connectionId);
  const startedAt = Date.now();
  const result = await executeSql(connection, payload.sql);
  return {
    ...result,
    connectionName: connection.name,
    sql: payload.sql,
    elapsedMs: Date.now() - startedAt,
    executedAt: new Date().toISOString(),
  };
});

ipcMain.handle("settings:get", () => readStore().settings);

ipcMain.handle("settings:save", (_event, settings) => {
  const store = readStore();
  store.settings = { ...store.settings, ...settings };
  writeStore(store);
  return store.settings;
});

app.whenReady().then(() => {
  app.setName("sqlg");
  createAppMenu();
  createWindow();
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});
