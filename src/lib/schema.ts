export type ColumnKind = "pk" | "fk" | "numeric" | "text" | "datetime" | "enum" | "other";

export type SchemaColumn = {
  name: string;
  dataType: string;
  columnType: string;
  columnKey: string;
  foreignKey: boolean;
};

export type SchemaTable = { name: string; columns: SchemaColumn[] };
export type SchemaGroup = { name: string; tables: SchemaTable[] };
export type SchemaSnapshot = { lowerCaseTableNames: number; schemas: SchemaGroup[] };

export function columnKind(column: SchemaColumn): ColumnKind {
  if (column.columnKey.toUpperCase() === "PRI") {
    return "pk";
  }
  if (column.foreignKey) {
    return "fk";
  }
  const data = column.dataType.toLowerCase();
  const full = column.columnType.toLowerCase();
  if (full.startsWith("tinyint(1)") || data === "bool" || data === "boolean" || data === "enum" || data === "set") {
    return "enum";
  }
  if (data === "year" || data.includes("date") || data.includes("time")) {
    return "datetime";
  }
  if (["int", "decimal", "numeric", "float", "double", "bit", "real", "serial"].some((needle) => data.includes(needle))) {
    return "numeric";
  }
  if (["char", "text", "blob", "binary", "json"].some((needle) => data.includes(needle))) {
    return "text";
  }
  return "other";
}

export function completionSchema(snapshot: SchemaSnapshot | undefined, defaultSchema: string | null): Record<string, unknown> {
  if (!snapshot) {
    return {};
  }
  const nested: Record<string, Record<string, string[]>> = {};
  for (const group of snapshot.schemas) {
    nested[group.name] = {};
    for (const table of group.tables) {
      nested[group.name][table.name] = table.columns.map((column) => column.name);
    }
  }
  const preferred = defaultSchema ? nested[defaultSchema] : snapshot.schemas.length === 1 ? nested[snapshot.schemas[0].name] : undefined;
  if (!preferred) {
    return nested;
  }
  return { ...nested, ...preferred };
}

export function lintCatalog(snapshot: SchemaSnapshot | undefined, defaultSchema: string | null) {
  return {
    defaultSchema,
    lowerCaseTableNames: snapshot?.lowerCaseTableNames ?? 0,
    schemas: (snapshot?.schemas ?? []).map((group) => ({
      name: group.name,
      tables: group.tables.map((table) => ({
        name: table.name,
        columns: table.columns.map((column) => column.name),
      })),
    })),
  };
}
