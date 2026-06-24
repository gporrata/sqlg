export type DbType = "postgres" | "mysql";
export type ThemeName = "system" | "light" | "dark" | "solarized" | "gruvbox" | "winter-solarized" | "ayu-light";
export type StaticTabName = "connections" | "editor" | "history";
export type ResultTabName = `result:${string}`;
export type TabName = StaticTabName | ResultTabName;

export type Connection = {
  id: string;
  name: string;
  type: DbType;
  host: string;
  port: number;
  database: string;
  defaultSchema?: string;
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
  defaultSchema?: string;
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

export type SchemaColumnMetadata = {
  name: string;
  dataType: string;
  nullable: boolean;
  defaultValue?: string;
  isPrimaryKey?: boolean;
  isUnique?: boolean;
  foreignKey?: string;
  extra?: string[];
};

export type SchemaTableMetadata = {
  schema: string;
  name: string;
  tableType: string;
  characteristics: string[];
  columns: SchemaColumnMetadata[];
};

export type ConnectionSchemaMetadata = {
  provider: DbType;
  database: string;
  schemas: {
    name: string;
    tables: SchemaTableMetadata[];
  }[];
};

export type Settings = {
  theme: ThemeName;
};
