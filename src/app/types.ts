import type { LayoutMode } from "../lib/layout";
import type { SchemaSnapshot } from "../lib/schema";

export type Connection = {
  name: string;
  host: string;
  port: number;
  user: string;
  schema?: string | null;
  ssl: boolean;
  explain_lint: boolean;
  color?: string | null;
};

export type ColumnInfo = { name: string; typeName: string; numeric: boolean };

export type ResultSection = {
  index: number;
  sql: string;
  columns: ColumnInfo[];
  rows: (string | null)[][];
  rowsAffected: number | null;
  rowCount: number;
  error: string | null;
  elapsedMs: number | null;
  done: boolean;
};

export type FooterState = {
  text: string;
  tone: "idle" | "ok" | "fail" | "run";
  elapsedMs: number | null;
};

export type Tab = {
  id: string;
  connectionName: string | null;
  sql: string;
  cursor: number;
  selectionAnchor: number | null;
  selectionHead: number | null;
  scrollTop: number;
  layout: LayoutMode;
  filePath: string | null;
  savedSql: string | null;
  newline: string;
  bom: boolean;
  results: ResultSection[];
  running: boolean;
  runId: string | null;
  footer: FooterState;
  flash: { from: number; to: number; token: number } | null;
  restored: boolean;
  externalRev: number;
  diskNotice: string | null;
};

export type TabSnapshot = {
  id: string;
  connectionName?: string | null;
  sql?: string;
  cursor?: number;
  selectionAnchor?: number | null;
  selectionHead?: number | null;
  scrollTop?: number;
  layout?: string;
  filePath?: string | null;
  savedSql?: string | null;
  newline?: string;
  dirty?: boolean;
};

export type WindowSnapshot = {
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  fullscreen?: boolean;
  order?: number;
  activeTab?: string;
  tabs: TabSnapshot[];
};

export type SessionDocument = {
  version?: number;
  theme?: string;
  frontWindow?: string | null;
  windows?: WindowSnapshot[];
};

export type QueryEvent =
  | { kind: "begin"; runId: string; index: number; total: number; sql: string }
  | { kind: "columns"; index: number; columns: ColumnInfo[] }
  | { kind: "rows"; index: number; rows: (string | null)[][] }
  | { kind: "statementDone"; index: number; rowsAffected: number | null; rowCount: number; elapsedMs: number }
  | { kind: "statementError"; index: number; message: string }
  | { kind: "finished"; ok: boolean; executed: number; total: number; failed: number; elapsedMs: number; cancelled: boolean };

export type CloseChoice = "save" | "discard" | "cancel";

export type SchemaMap = Record<string, SchemaSnapshot>;

export function emptyFooter(): FooterState {
  return { text: "", tone: "idle", elapsedMs: null };
}

export function makeTab(connectionName: string | null, partial: Partial<Tab> = {}): Tab {
  return {
    id: crypto.randomUUID(),
    connectionName,
    sql: "",
    cursor: 0,
    selectionAnchor: null,
    selectionHead: null,
    scrollTop: 0,
    layout: "all",
    filePath: null,
    savedSql: null,
    newline: "\n",
    bom: false,
    results: [],
    running: false,
    runId: null,
    footer: emptyFooter(),
    flash: null,
    restored: false,
    externalRev: 0,
    diskNotice: null,
    ...partial,
  };
}
