const { app, BrowserWindow, Menu, ipcMain, nativeImage } = require("electron");
const path = require("node:path");
const fs = require("node:fs");
const mysql = require("mysql2/promise");
const { Client } = require("pg");

const isDev = process.env.NODE_ENV !== "production";
const appIconPath = path.join(__dirname, "..", "assets", "sqlg-icon.png");
const schemaCache = new Map();
const activeQueries = new Map();

app.setName("sqlg");
app.setAppUserModelId("com.sqlg.app");
if (process.platform === "linux" && typeof app.setDesktopName === "function") {
  app.setDesktopName("sqlg.desktop");
}

function storePath() {
  return path.join(app.getPath("home"), ".config", "sqlg", "config.json");
}

function emptyStore() {
  return {
    connections: [],
    queryHistory: [],
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

function queryHistoryFor(connectionId) {
  return readStore()
    .queryHistory.filter((entry) => entry.connectionId === connectionId)
    .sort((left, right) => new Date(right.executedAt).getTime() - new Date(left.executedAt).getTime());
}

function addQueryHistoryEntry(entry) {
  const store = readStore();
  store.queryHistory = [entry, ...store.queryHistory].slice(0, 1000);
  writeStore(store);
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

function postgresConnectionOptions(connection) {
  return {
    host: connection.host,
    port: connection.port,
    database: connection.database,
    user: connection.user,
    password: connection.password,
  };
}

async function cancelPostgresQuery(connection, processId) {
  const client = new Client(postgresConnectionOptions(connection));
  await client.connect();
  try {
    await client.query("select pg_cancel_backend($1)", [processId]);
  } finally {
    await client.end();
  }
}

async function cancelMysqlQuery(connection, threadId) {
  const client = await mysql.createConnection({
    host: connection.host,
    port: connection.port,
    database: connection.database,
    user: connection.user,
    password: connection.password,
    multipleStatements: false,
  });
  try {
    await client.query(`KILL QUERY ${Number(threadId)}`);
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
      att.attname as column,
      pg_catalog.format_type(att.atttypid, att.atttypmod) as data_type,
      not att.attnotnull as nullable,
      pg_get_expr(def.adbin, def.adrelid) as default_value,
      att.attidentity as identity,
      att.attgenerated as generated,
      col_description(cls.oid, att.attnum) as comment
    from pg_attribute att
    join pg_class cls on cls.oid = att.attrelid
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    left join pg_attrdef def on def.adrelid = att.attrelid and def.adnum = att.attnum
    where att.attnum > 0
      and not att.attisdropped
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
  `);
  const tableResult = await client.query(`
    select
      nsp.nspname as schema,
      cls.relname as name,
      case cls.relkind
        when 'r' then 'table'
        when 'p' then 'partitioned table'
        when 'v' then 'view'
        when 'm' then 'materialized view'
        when 'f' then 'foreign table'
        else cls.relkind::text
      end as table_type,
      case cls.relpersistence
        when 'p' then 'permanent'
        when 'u' then 'unlogged'
        when 't' then 'temporary'
        else cls.relpersistence::text
      end as persistence,
      greatest(cls.reltuples::bigint, 0) as estimated_rows,
      obj_description(cls.oid, 'pg_class') as comment
    from pg_class cls
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    where cls.relkind in ('r', 'p', 'v', 'm', 'f')
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
    order by nsp.nspname, cls.relname
  `);
  const primaryKeyResult = await client.query(`
    select
      nsp.nspname as schema,
      cls.relname as table,
      array_agg(att.attname order by key.ord) as columns
    from pg_constraint con
    join pg_class cls on cls.oid = con.conrelid
    join pg_namespace nsp on nsp.oid = cls.relnamespace
    join unnest(con.conkey) with ordinality as key(attnum, ord) on true
    join pg_attribute att on att.attrelid = con.conrelid and att.attnum = key.attnum
    where con.contype = 'p'
      and nsp.nspname not in ('pg_catalog', 'information_schema')
      and nsp.nspname not like 'pg_toast%'
    group by con.oid, nsp.nspname, cls.relname
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
    tables: tableResult.rows.map((row) => ({
      schema: row.schema,
      name: row.name,
      tableType: row.table_type,
      characteristics: [
        `persistence: ${row.persistence}`,
        `estimated rows: ${row.estimated_rows}`,
        ...(row.comment ? [`comment: ${row.comment}`] : []),
      ],
      columns: columnResult.rows
        .filter((column) => column.schema === row.schema && column.table === row.name)
        .map((column) => ({
          name: column.column,
          dataType: column.data_type,
          nullable: Boolean(column.nullable),
          defaultValue: column.default_value || undefined,
          extra: [
            ...(column.identity ? [`identity: ${column.identity}`] : []),
            ...(column.generated ? [`generated: ${column.generated}`] : []),
            ...(column.comment ? [`comment: ${column.comment}`] : []),
          ],
        })),
    })),
    primaryKeys: primaryKeyResult.rows.map((row) => ({ schema: row.schema, table: row.table, columns: row.columns })),
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
  const [tables] = await client.query(`
    select
      table_schema as \`schema\`,
      table_name as name,
      table_type,
      engine,
      table_rows,
      row_format,
      table_collation,
      table_comment
    from information_schema.tables
    where table_schema = database()
    order by table_schema, table_name
  `);
  const [columns] = await client.query(`
    select
      table_schema as \`schema\`,
      table_name as \`table\`,
      column_name as name,
      column_type as data_type,
      is_nullable,
      column_default as default_value,
      column_key,
      extra,
      column_comment
    from information_schema.columns
    where table_schema = database()
    order by table_schema, table_name, ordinal_position
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
  const groupedPrimaryKeys = new Map();
  for (const row of uniqueKeys) {
    const key = `${row.schema}.${row.table}.${row.constraint_name}`;
    const current = groupedUniqueKeys.get(key) ?? { schema: row.schema, table: row.table, columns: [] };
    current.columns.push(row.column);
    groupedUniqueKeys.set(key, current);
    if (row.constraint_name === "PRIMARY") {
      const primaryKey = `${row.schema}.${row.table}`;
      const primary = groupedPrimaryKeys.get(primaryKey) ?? { schema: row.schema, table: row.table, columns: [] };
      primary.columns.push(row.column);
      groupedPrimaryKeys.set(primaryKey, primary);
    }
  }

  return {
    foreignKeys: [...groupedForeignKeys.values()],
    uniqueKeys: [...groupedUniqueKeys.values()],
    columnsById: {},
    tables: tables.map((row) => ({
      schema: row.schema,
      name: row.name,
      tableType: String(row.table_type || "table").toLowerCase(),
      characteristics: [
        ...(row.engine ? [`engine: ${row.engine}`] : []),
        ...(row.row_format ? [`row format: ${row.row_format}`] : []),
        ...(row.table_collation ? [`collation: ${row.table_collation}`] : []),
        ...(row.table_rows !== null && row.table_rows !== undefined ? [`estimated rows: ${row.table_rows}`] : []),
        ...(row.table_comment ? [`comment: ${row.table_comment}`] : []),
      ],
      columns: columns
        .filter((column) => column.schema === row.schema && column.table === row.name)
        .map((column) => ({
          name: column.name,
          dataType: column.data_type,
          nullable: column.is_nullable === "YES",
          defaultValue: column.default_value ?? undefined,
          extra: [
            ...(column.extra ? [column.extra] : []),
            ...(column.column_key ? [`key: ${column.column_key}`] : []),
            ...(column.column_comment ? [`comment: ${column.column_comment}`] : []),
          ],
        })),
    })),
    primaryKeys: [...groupedPrimaryKeys.values()],
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
    foreignKeys: schema.foreignKeys.map((foreignKey) => ({
      ...foreignKey,
      fromColumns: listValue(foreignKey.fromColumns),
      toColumns: listValue(foreignKey.toColumns),
    })),
    uniqueKeys: schema.uniqueKeys.map((key) => ({
      ...key,
      columns: listValue(key.columns),
    })),
  };
}

function columnKey(schema, table, column) {
  return `${schema}.${table}.${column}`;
}

function listValue(value) {
  if (Array.isArray(value)) {
    return value;
  }
  if (value === null || value === undefined) {
    return [];
  }
  if (typeof value === "string" && value.startsWith("{") && value.endsWith("}")) {
    return value.slice(1, -1).split(",").filter(Boolean);
  }
  return [value];
}

function publicConnectionSchema(connection, schema) {
  const primaryColumns = new Set(
    (schema.primaryKeys ?? []).flatMap((key) => listValue(key.columns).map((column) => columnKey(key.schema, key.table, column))),
  );
  const uniqueColumns = new Set(
    schema.uniqueKeys
      .filter((key) => listValue(key.columns).length === 1)
      .flatMap((key) => listValue(key.columns).map((column) => columnKey(key.schema, key.table, column))),
  );
  const foreignColumns = new Map();
  for (const foreignKey of schema.foreignKeys) {
    const toColumns = listValue(foreignKey.toColumns);
    listValue(foreignKey.fromColumns).forEach((fromColumn, index) => {
      foreignColumns.set(
        columnKey(foreignKey.fromSchema, foreignKey.fromTable, fromColumn),
        `${foreignKey.toSchema}.${foreignKey.toTable}.${toColumns[index]}`,
      );
    });
  }
  const schemas = new Map();
  for (const table of schema.tables ?? []) {
    const current = schemas.get(table.schema) ?? { name: table.schema, tables: [] };
    current.tables.push({
      ...table,
      columns: listValue(table.columns).map((column) => {
        const key = columnKey(table.schema, table.name, column.name);
        return {
          ...column,
          isPrimaryKey: primaryColumns.has(key),
          isUnique: uniqueColumns.has(key),
          foreignKey: foreignColumns.get(key),
        };
      }),
    });
    schemas.set(table.schema, current);
  }
  return {
    provider: connection.type,
    database: connection.database,
    schemas: [...schemas.values()],
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

async function executeSql(connection, sql, queryId) {
  if (!sql.trim()) {
    throw new Error("No SQL to execute");
  }

  return withClient(connection, async (client) => {
    if (queryId) {
      const cancel = connection.type === "mysql"
        ? () => cancelMysqlQuery(connection, client.threadId)
        : () => cancelPostgresQuery(connection, client.processID);
      activeQueries.set(queryId, { cancel });
    }
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
  }).finally(() => {
    if (queryId) {
      activeQueries.delete(queryId);
    }
  });
}

function createWindow() {
  const appIcon = nativeImage.createFromPath(appIconPath);
  const win = new BrowserWindow({
    width: 1280,
    height: 820,
    minWidth: 920,
    minHeight: 640,
    title: "sqlg",
    icon: appIcon,
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
  store.queryHistory = store.queryHistory.filter((item) => item.connectionId !== id);
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

ipcMain.handle("schema:load", async (_event, payload) => {
  const connectionId = typeof payload === "string" ? payload : payload.connectionId;
  const connection = findConnection(connectionId);
  if (payload.refresh) {
    schemaCache.delete(connection.id);
  }
  return withClient(connection, async (client) => {
    const schema = await getSchema(connection, client);
    return publicConnectionSchema(connection, schema);
  });
});

ipcMain.handle("query:execute", async (_event, payload) => {
  const connection = findConnection(payload.connectionId);
  const queryId = payload.queryId || `${payload.connectionId}-${Date.now()}`;
  const startedAt = Date.now();
  const executedAt = new Date().toISOString();
  const baseHistoryEntry = {
    id: queryId,
    connectionId: payload.connectionId,
    connectionName: connection.name,
    sql: payload.sql,
    executedAt,
  };
  try {
    const result = await executeSql(connection, payload.sql, queryId);
    const elapsedMs = Date.now() - startedAt;
    addQueryHistoryEntry({
      ...baseHistoryEntry,
      rowCount: result.rowCount,
      elapsedMs,
      status: "success",
    });
    return {
      ...result,
      connectionName: connection.name,
      sql: payload.sql,
      elapsedMs,
      executedAt,
    };
  } catch (error) {
    addQueryHistoryEntry({
      ...baseHistoryEntry,
      elapsedMs: Date.now() - startedAt,
      status: "error",
      error: error instanceof Error ? error.message : String(error),
    });
    throw error;
  }
});

ipcMain.handle("query:cancel", async (_event, queryId) => {
  const activeQuery = activeQueries.get(queryId);
  if (!activeQuery) {
    return false;
  }
  await activeQuery.cancel();
  return true;
});

ipcMain.handle("history:list", (_event, connectionId) => queryHistoryFor(connectionId));

ipcMain.handle("settings:get", () => readStore().settings);

ipcMain.handle("settings:save", (_event, settings) => {
  const store = readStore();
  store.settings = { ...store.settings, ...settings };
  writeStore(store);
  return store.settings;
});

app.whenReady().then(() => {
  app.setName("sqlg");
  if (process.platform === "darwin") {
    app.dock.setIcon(appIconPath);
  }
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
