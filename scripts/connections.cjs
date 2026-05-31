#!/usr/bin/env node

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function storePath() {
  return path.join(os.homedir(), ".config", "sqlg", "config.json");
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

function usage() {
  console.log(`Usage:
  sqlg add <name> <mysql|postgres> <uri>
  sqlg list
  sqlg remove <name>

SQLG_USERNAME and SQLG_PASSWORD are used for credentials when adding a connection.`);
}

function defaultPort(type) {
  return type === "mysql" ? 33306 : 55432;
}

function parseConnectionUri(type, rawUri) {
  const uri = rawUri.includes("://") ? rawUri : `${type}://${rawUri}`;
  const parsed = new URL(uri);
  return {
    host: parsed.hostname || "127.0.0.1",
    port: parsed.port ? Number(parsed.port) : defaultPort(type),
    database: parsed.pathname.replace(/^\/+/, "") || "sqlg",
  };
}

function addConnection(args) {
  const [name, provider, uri] = args;
  const type = provider === "mysql" ? "mysql" : provider === "postgres" ? "postgres" : "";
  if (!name || !type || !uri) {
    usage();
    process.exit(1);
  }
  if (!process.env.SQLG_USERNAME || !process.env.SQLG_PASSWORD) {
    console.error("SQLG_USERNAME and SQLG_PASSWORD must be set.");
    process.exit(1);
  }

  const store = readStore();
  const existing = store.connections.find((connection) => connection.name === name);
  const parsed = parseConnectionUri(type, uri);
  const connection = {
    id: existing?.id || `${type}-${Date.now()}`,
    name,
    type,
    ...parsed,
    user: process.env.SQLG_USERNAME,
    password: process.env.SQLG_PASSWORD,
  };

  store.connections = [...store.connections.filter((item) => item.name !== name), connection];
  writeStore(store);
  console.log(`Saved ${name}`);
}

function listConnections() {
  const { connections } = readStore();
  if (!connections.length) {
    console.log("No connections");
    return;
  }
  for (const connection of connections) {
    console.log(`${connection.name}\t${connection.type}://${connection.user}@${connection.host}:${connection.port}/${connection.database}`);
  }
}

function removeConnection(args) {
  const [name] = args;
  if (!name) {
    usage();
    process.exit(1);
  }
  const store = readStore();
  const nextConnections = store.connections.filter((connection) => connection.name !== name);
  if (nextConnections.length === store.connections.length) {
    console.error(`Connection not found: ${name}`);
    process.exit(1);
  }
  store.connections = nextConnections;
  writeStore(store);
  console.log(`Removed ${name}`);
}

const [command, ...args] = process.argv.slice(2);
if (command === "add") {
  addConnection(args);
} else if (command === "list") {
  listConnections();
} else if (command === "remove") {
  removeConnection(args);
} else {
  usage();
  process.exit(command ? 1 : 0);
}
