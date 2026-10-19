import { invoke, Channel } from "@tauri-apps/api/core";
import type { Connection, QueryEvent, SessionDocument, WindowSnapshot } from "./types";
import type { SchemaSnapshot } from "../lib/schema";

export type TextFile = { text: string; newline: string; bom: boolean };
export type ScratchTab = {
  id: string;
  window?: string;
  connectionName?: string | null;
  sql: string;
  cursor?: number;
  layout?: string;
};
export type StatementRange = { sql: string; start: number; end: number };

export const api = {
  listConnections: () => invoke<Connection[]>("list_connections"),
  addConnection: (input: Record<string, unknown>) => invoke<void>("add_connection", input),
  updateConnection: (input: Record<string, unknown>) => invoke<Connection>("update_connection", input),
  removeConnection: (name: string) => invoke<void>("remove_connection", { name }),
  testConnection: (input: Record<string, unknown>) => invoke<string>("test_connection", input),
  loadSchema: (name: string) => invoke<SchemaSnapshot>("load_schema", { name }),
  ping: (name: string) => invoke<boolean>("connection_ping", { name }),
  runQuery: (connectionName: string, sql: string, channel: Channel<QueryEvent>) =>
    invoke<string>("run_query", { connectionName, sql, channel }),
  cancelQuery: (runId: string) => invoke<void>("cancel_query", { runId }),
  forgetRun: (runId: string) => invoke<void>("forget_run", { runId }),
  lint: (sql: string, catalog: unknown) => invoke<Diagnostic[]>("lint_sql_cmd", { sql, catalog }),
  explain: (name: string, sql: string) => invoke<string>("explain_sql", { name, sql }),
  statementAt: (sql: string, cursor: number) => invoke<StatementRange | null>("statement_at_cmd", { sql, cursor }),
  readText: (path: string) => invoke<TextFile>("read_text_file", { path }),
  writeText: (path: string, text: string, newline: string, bom: boolean) =>
    invoke<void>("write_text_file", { path, text, newline, bom }),
  watchFile: (path: string) => invoke<void>("watch_file", { path }),
  unwatchFile: (path: string) => invoke<void>("unwatch_file", { path }),
  loadSession: () => invoke<SessionDocument>("load_session"),
  upsertWindow: (snapshot: WindowSnapshot) => invoke<void>("upsert_window", { snapshot }),
  loadScratch: () => invoke<ScratchTab[]>("load_scratch"),
  saveScratch: (window: string, tabs: ScratchTab[]) => invoke<void>("save_scratch", { window, tabs }),
  holdQuit: (hold: boolean) => invoke<void>("hold_quit", { hold }),
  cancelQuit: () => invoke<void>("cancel_quit"),
  removeWindow: (label: string) => invoke<void>("remove_window", { label }),
  setTheme: (theme: string) => invoke<void>("set_theme", { theme }),
  createWindow: () => invoke<string>("create_window"),
  takePending: (label: string) => invoke<CliPending | null>("take_pending", { label }),
  isQuitting: () => invoke<boolean>("is_quitting"),
  ackQuit: () => invoke<void>("ack_quit"),
  permitClose: (label: string) => invoke<void>("permit_close", { label }),
  installCli: () => invoke<string>("install_cli"),
  uninstallCli: () => invoke<string>("uninstall_cli"),
};

export type CliPending = {
  conn: string;
  sql: string;
  target: string;
  title?: string | null;
  run: boolean;
  ack: string;
};

export type Diagnostic = {
  from: number;
  to: number;
  severity: string;
  message: string;
  actions: { name: string; text: string }[];
};
