export type DbType = "postgres" | "mysql";
export type ThemeName = "system" | "light" | "dark" | "solarized";
export type TabName = "connections" | "editor" | "results";

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
  rowCount: number;
  connectionName: string;
  sql: string;
  elapsedMs: number;
  executedAt: string;
};

export type Settings = {
  theme: ThemeName;
};
