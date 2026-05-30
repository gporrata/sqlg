export type DbType = "postgres" | "mysql";
export type ThemeName = "system" | "light" | "dark" | "solarized" | "gruvbox" | "winter-solarized" | "ayu-light";
export type TabName = "connections" | "editor" | "results" | "history";

export type Connection = {
  id: string;
  name: string;
  type: DbType;
  host: string;
  port: number;
  database: string;
  user: string;
  hasPassword?: boolean;
};

export type ConnectionInput = Omit<Connection, "id" | "hasPassword"> & {
  id?: string;
  password: string;
};

export type QueryResult = {
  rows: Record<string, unknown>[];
  fields: string[];
  columns: ResultColumn[];
  schema: SchemaMetadata;
  rowCount: number;
  connectionName: string;
  sql: string;
  elapsedMs: number;
  executedAt: string;
};

export type QueryHistoryEntry = {
  id: string;
  connectionId: string;
  connectionName: string;
  sql: string;
  rowCount?: number;
  elapsedMs: number;
  executedAt: string;
  status: "success" | "error";
  error?: string;
};

export type ResultColumn = {
  name: string;
  schema?: string;
  table?: string;
  column?: string;
};

export type SchemaForeignKey = {
  fromSchema: string;
  fromTable: string;
  fromColumns: string[];
  toSchema: string;
  toTable: string;
  toColumns: string[];
};

export type SchemaUniqueKey = {
  schema: string;
  table: string;
  columns: string[];
};

export type SchemaMetadata = {
  foreignKeys: SchemaForeignKey[];
  uniqueKeys: SchemaUniqueKey[];
};

export type Settings = {
  theme: ThemeName;
};
