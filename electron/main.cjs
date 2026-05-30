const { app, BrowserWindow, ipcMain } = require("electron");
const path = require("node:path");
const fs = require("node:fs");
const mysql = require("mysql2/promise");
const { Client } = require("pg");

const isDev = process.env.NODE_ENV !== "production";

function storePath() {
  return path.join(app.getPath("userData"), "sqlg-poc-store.json");
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

async function executeSql(connection, sql) {
  if (!sql.trim()) {
    throw new Error("No SQL to execute");
  }

  return withClient(connection, async (client) => {
    if (connection.type === "mysql") {
      const [rows, fields] = await client.query(sql);
      return {
        rows: Array.isArray(rows) ? rows : [],
        fields: fields.map((field) => field.name),
        rowCount: Array.isArray(rows) ? rows.length : 0,
      };
    }

    const result = await client.query(sql);
    return {
      rows: result.rows,
      fields: result.fields.map((field) => field.name),
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
    title: "sqlg Electron POC",
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
  writeStore(store);
  return publicConnection(connection);
});

ipcMain.handle("connections:delete", (_event, id) => {
  const store = readStore();
  store.connections = store.connections.filter((item) => item.id !== id);
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

app.whenReady().then(createWindow);

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
