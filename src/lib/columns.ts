import type { SchemaSnapshot } from "./schema";

export type GroupedColumn = { index: number; name: string; typeName: string };

export type ColumnGroup = { alias: string; columns: GroupedColumn[] };

type Tok = { t: string; k: "id" | "sym" | "star" | "str" };

type TableRef = { alias: string; schema: string | null; table: string | null };

type Slot = { kind: "col"; group: string; source: string } | { kind: "star"; group: string | null } | { kind: "expr" };

const CLAUSE = new Set([
  "where",
  "group",
  "order",
  "limit",
  "union",
  "except",
  "intersect",
  "having",
  "window",
  "for",
  "lock",
  "into",
  "set",
  "procedure",
  "qualify",
]);

const SELECT_MOD = new Set([
  "distinct",
  "distinctrow",
  "all",
  "high_priority",
  "straight_join",
  "sql_small_result",
  "sql_big_result",
  "sql_buffer_result",
  "sql_no_cache",
  "sql_calc_found_rows",
  "sql_cache",
]);

export function columnGroups(
  sql: string,
  columns: { name: string; typeName: string }[],
  schema?: SchemaSnapshot | null,
  defaultSchema?: string | null,
): ColumnGroup[] {
  const indexed = columns.map((column, index) => ({ index, name: column.name, typeName: column.typeName }));
  const parsed = parseSelect(sql);
  const labels = parsed
    ? align(parsed.slots, indexed.length, parsed.tables, schema, defaultSchema)
    : indexed.map(() => "columns");
  const order: string[] = [];
  const groups = new Map<string, GroupedColumn[]>();
  labels.forEach((alias, index) => {
    const label = alias || "columns";
    const column = indexed[index];
    if (!column) return;
    if (!groups.has(label)) {
      groups.set(label, []);
      order.push(label);
    }
    groups.get(label)?.push(column);
  });
  return order.map((alias) => ({ alias, columns: groups.get(alias) ?? [] }));
}

function align(
  slots: Slot[],
  count: number,
  tables: TableRef[],
  schema?: SchemaSnapshot | null,
  defaultSchema?: string | null,
): string[] {
  const expanded = expandSlots(slots, tables, schema, defaultSchema);
  if (expanded && expanded.length === count) return expanded;
  if (slots.every((slot) => slot.kind !== "star") && slots.length === count) {
    return slots.map((slot) => slotGroup(slot, tables, schema, defaultSchema));
  }
  const stars = slots.filter((slot) => slot.kind === "star");
  if (stars.length === 1) {
    const starAt = slots.findIndex((slot) => slot.kind === "star");
    const before = slots.slice(0, starAt);
    const after = slots.slice(starAt + 1);
    if (before.length + after.length <= count) {
      const star = slots[starAt];
      const groups = new Array<string>(count).fill(star ? slotGroup(star, tables, schema, defaultSchema) : "columns");
      before.forEach((slot, index) => {
        groups[index] = slotGroup(slot, tables, schema, defaultSchema);
      });
      after.forEach((slot, index) => {
        groups[count - after.length + index] = slotGroup(slot, tables, schema, defaultSchema);
      });
      return groups;
    }
  }
  return new Array<string>(count).fill("columns");
}

function expandSlots(
  slots: Slot[],
  tables: TableRef[],
  schema?: SchemaSnapshot | null,
  defaultSchema?: string | null,
): string[] | null {
  if (!schema) return null;
  const out: string[] = [];
  for (const slot of slots) {
    if (slot.kind !== "star") {
      out.push(slotGroup(slot, tables, schema, defaultSchema));
      continue;
    }
    const cols = expandStar(slot.group, tables, schema, defaultSchema);
    if (!cols) return null;
    out.push(...cols);
  }
  return out;
}

function expandStar(
  group: string | null,
  tables: TableRef[],
  schema: SchemaSnapshot,
  defaultSchema?: string | null,
): string[] | null {
  const targets = group ? tables.filter((table) => table.alias.toLowerCase() === group.toLowerCase()) : tables.filter((table) => table.alias);
  if (targets.length === 0) return null;
  const out: string[] = [];
  for (const table of targets) {
    const cols = lookupColumns(schema, table, defaultSchema);
    if (!cols) return null;
    for (let index = 0; index < cols.length; index += 1) out.push(table.alias);
  }
  return out;
}

function slotGroup(slot: Slot, tables: TableRef[], schema?: SchemaSnapshot | null, defaultSchema?: string | null): string {
  if (slot.kind === "expr") return "expr";
  if (slot.kind === "star") {
    if (slot.group) return normalizeGroup(slot.group, tables);
    const named = tables.filter((table) => table.alias);
    return named.length === 1 ? named[0].alias : "columns";
  }
  if (slot.group) return normalizeGroup(slot.group, tables);
  const named = tables.filter((table) => table.alias);
  if (named.length === 1) return named[0].alias;
  const owner = schema ? uniqueOwner(slot.source, named, schema, defaultSchema) : null;
  return owner ?? "columns";
}

function normalizeGroup(group: string, tables: TableRef[]): string {
  const byAlias = tables.find((table) => table.alias.toLowerCase() === group.toLowerCase());
  if (byAlias) return byAlias.alias;
  const byTable = tables.filter((table) => table.table?.toLowerCase() === group.toLowerCase());
  return byTable.length === 1 ? byTable[0].alias : group;
}

function uniqueOwner(name: string, tables: TableRef[], schema: SchemaSnapshot, defaultSchema?: string | null): string | null {
  const owners = tables.filter((table) => {
    const cols = lookupColumns(schema, table, defaultSchema);
    return cols?.some((column) => column.toLowerCase() === name.toLowerCase());
  });
  return owners.length === 1 ? owners[0].alias : null;
}

function lookupColumns(schema: SchemaSnapshot, table: TableRef, defaultSchema?: string | null): string[] | null {
  if (!table.table) return null;
  const mode = schema.lowerCaseTableNames;
  const wantSchema = table.schema ?? defaultSchema ?? null;
  const groups = wantSchema ? schema.schemas.filter((group) => sameIdent(group.name, wantSchema, mode)) : schema.schemas;
  const found: string[][] = [];
  for (const group of groups) {
    for (const item of group.tables) {
      if (sameIdent(item.name, table.table, mode)) found.push(item.columns.map((column) => column.name));
    }
  }
  return found.length === 1 ? found[0] : null;
}

function sameIdent(left: string, right: string, mode: number): boolean {
  if (mode === 0) return left === right;
  return left.toLowerCase() === right.toLowerCase();
}

function parseSelect(sql: string): { slots: Slot[]; tables: TableRef[] } | null {
  const tokens = lex(sql);
  let depth = 0;
  let selectAt = -1;
  for (let index = 0; index < tokens.length; index += 1) {
    const tok = tokens[index];
    if (tok.t === "(") depth += 1;
    else if (tok.t === ")") depth = Math.max(0, depth - 1);
    else if (depth === 0 && wordIs(tok, "select")) {
      selectAt = index;
      break;
    }
  }
  if (selectAt < 0) return null;
  let index = selectAt + 1;
  while (wordIn(tokens[index], SELECT_MOD)) index += 1;
  const items: Tok[][] = [];
  let current: Tok[] = [];
  depth = 0;
  let fromAt = -1;
  for (; index < tokens.length; index += 1) {
    const tok = tokens[index];
    if (tok.t === "(") {
      depth += 1;
      current.push(tok);
      continue;
    }
    if (tok.t === ")") {
      depth = Math.max(0, depth - 1);
      current.push(tok);
      continue;
    }
    if (depth === 0 && tok.t === ",") {
      if (current.length) items.push(current);
      current = [];
      continue;
    }
    if (depth === 0 && (wordIs(tok, "from") || wordIn(tok, CLAUSE))) {
      if (wordIs(tok, "from")) fromAt = index;
      break;
    }
    current.push(tok);
  }
  if (current.length) items.push(current);
  return {
    slots: items.map(parseItem),
    tables: fromAt >= 0 ? parseTables(tokens, fromAt + 1) : [],
  };
}

function parseItem(toks: Tok[]): Slot {
  const body = stripAlias(toks);
  if (body.length === 1 && body[0].k === "star") return { kind: "star", group: null };
  if (body.length >= 3 && body[body.length - 1].k === "star" && body[body.length - 2].t === "." && body[body.length - 3].k === "id") {
    return { kind: "star", group: body[body.length - 3].t };
  }
  if (isColumnRef(body)) {
    const parts = body.filter((tok) => tok.k === "id").map((tok) => tok.t);
    if (parts.length <= 1) return { kind: "col", group: "", source: parts[0] ?? "" };
    return { kind: "col", group: parts[parts.length - 2] ?? "", source: parts[parts.length - 1] ?? "" };
  }
  return { kind: "expr" };
}

function stripAlias(toks: Tok[]): Tok[] {
  const explicit = depthZeroAs(toks);
  if (explicit >= 0 && explicit === toks.length - 2 && toks[explicit + 1]?.k === "id") {
    return stripImplicit(toks.slice(0, explicit));
  }
  return stripImplicit(toks);
}

function stripImplicit(body: Tok[]): Tok[] {
  if (body.length < 2 || body[body.length - 1]?.k !== "id") return body;
  const head = body.slice(0, -1);
  const tail = head[head.length - 1];
  if (isColumnRef(head) || tail?.t === ")" || tail?.k === "str") return head;
  return body;
}

function depthZeroAs(toks: Tok[]): number {
  let depth = 0;
  let at = -1;
  for (let index = 0; index < toks.length; index += 1) {
    const tok = toks[index];
    if (tok.t === "(") depth += 1;
    else if (tok.t === ")") depth = Math.max(0, depth - 1);
    else if (depth === 0 && wordIs(tok, "as")) at = index;
  }
  return at;
}

function isColumnRef(toks: Tok[]): boolean {
  if (toks.length === 1) return toks[0].k === "id";
  if (toks.length === 3) return toks[0].k === "id" && toks[1].t === "." && toks[2].k === "id";
  if (toks.length === 5) {
    return toks[0].k === "id" && toks[1].t === "." && toks[2].k === "id" && toks[3].t === "." && toks[4].k === "id";
  }
  return false;
}

function parseTables(tokens: Tok[], start: number): TableRef[] {
  const tables: TableRef[] = [];
  let index = start;
  while (index < tokens.length) {
    const tok = tokens[index];
    if (wordIn(tok, CLAUSE)) break;
    if (tok.t === ",") {
      index += 1;
      continue;
    }
    if (startsJoin(tokens, index)) {
      index = skipJoin(tokens, index);
      continue;
    }
    if (wordIs(tok, "on") || wordIs(tok, "using")) {
      index = skipCondition(tokens, index + 1);
      continue;
    }
    if (wordIs(tok, "use") || wordIs(tok, "ignore") || wordIs(tok, "force")) {
      index = skipHint(tokens, index);
      continue;
    }
    if (wordIs(tok, "partition") && tokens[index + 1]?.t === "(") {
      index = skipParens(tokens, index + 1);
      continue;
    }
    if (tok.t === "(") {
      const after = skipParens(tokens, index);
      const alias = readAlias(tokens, after);
      tables.push({ alias: alias.name ?? "", schema: null, table: null });
      index = alias.next;
      continue;
    }
    const name = readName(tokens, index);
    if (!name) {
      index += 1;
      continue;
    }
    const alias = readAlias(tokens, name.next);
    const table = name.parts[name.parts.length - 1] ?? "";
    tables.push({
      alias: alias.name ?? table,
      schema: name.parts.length >= 2 ? (name.parts[name.parts.length - 2] ?? null) : null,
      table,
    });
    index = alias.next;
  }
  return tables;
}

function readName(tokens: Tok[], index: number): { parts: string[]; next: number } | null {
  if (tokens[index]?.k !== "id") return null;
  const parts = [tokens[index].t];
  let next = index + 1;
  while (tokens[next]?.t === "." && tokens[next + 1]?.k === "id") {
    parts.push(tokens[next + 1].t);
    next += 2;
  }
  return { parts, next };
}

function readAlias(tokens: Tok[], index: number): { name: string | null; next: number } {
  if (wordIs(tokens[index], "as") && tokens[index + 1]?.k === "id") {
    return { name: tokens[index + 1].t, next: index + 2 };
  }
  const tok = tokens[index];
  if (tok?.k === "id" && !wordIn(tok, CLAUSE) && !startsJoin(tokens, index) && !wordIs(tok, "on") && !wordIs(tok, "using")) {
    return { name: tok.t, next: index + 1 };
  }
  return { name: null, next: index };
}

function skipCondition(tokens: Tok[], start: number): number {
  let depth = 0;
  let index = start;
  while (index < tokens.length) {
    const tok = tokens[index];
    if (tok.t === "(") depth += 1;
    else if (tok.t === ")") {
      if (depth === 0) break;
      depth -= 1;
    } else if (depth === 0 && (tok.t === "," || startsJoin(tokens, index) || wordIn(tok, CLAUSE))) {
      break;
    }
    index += 1;
  }
  return index;
}

function skipJoin(tokens: Tok[], index: number): number {
  let next = index;
  while (next < tokens.length && !wordIs(tokens[next], "join") && !wordIs(tokens[next], "straight_join")) next += 1;
  return Math.min(tokens.length, next + 1);
}

function skipHint(tokens: Tok[], index: number): number {
  let next = index + 1;
  if (wordIs(tokens[next], "index") || wordIs(tokens[next], "key") || wordIs(tokens[next], "indexes") || wordIs(tokens[next], "keys")) {
    next += 1;
  }
  if (wordIs(tokens[next], "for")) {
    next += 1;
    if (wordIs(tokens[next], "join") || wordIs(tokens[next], "order") || wordIs(tokens[next], "group")) next += 1;
    if (wordIs(tokens[next], "by")) next += 1;
  }
  if (tokens[next]?.t === "(") return skipParens(tokens, next);
  return next;
}

function skipParens(tokens: Tok[], index: number): number {
  let depth = 0;
  let next = index;
  while (next < tokens.length) {
    if (tokens[next].t === "(") depth += 1;
    else if (tokens[next].t === ")") {
      depth -= 1;
      next += 1;
      if (depth === 0) break;
      continue;
    }
    next += 1;
  }
  return next;
}

function startsJoin(tokens: Tok[], index: number): boolean {
  const word = lower(tokens[index]);
  if (!word) return false;
  if (word === "join" || word === "straight_join") return true;
  if (!["inner", "cross", "natural", "full", "left", "right", "outer"].includes(word)) return false;
  for (let next = index + 1; next < tokens.length && next < index + 5; next += 1) {
    const follow = lower(tokens[next]);
    if (follow === "join") return true;
    if (!["outer", "inner", "left", "right", "full", "cross"].includes(follow ?? "")) return false;
  }
  return false;
}

function wordIs(tok: Tok | undefined, word: string): boolean {
  return tok?.k === "id" && tok.t.toLowerCase() === word;
}

function wordIn(tok: Tok | undefined, words: Set<string>): boolean {
  return tok?.k === "id" && words.has(tok.t.toLowerCase());
}

function lower(tok: Tok | undefined): string | null {
  return tok?.k === "id" ? tok.t.toLowerCase() : null;
}

function lex(sql: string): Tok[] {
  const out: Tok[] = [];
  let index = 0;
  while (index < sql.length) {
    const ch = sql[index] ?? "";
    if (ch === " " || ch === "\n" || ch === "\t" || ch === "\r" || ch === "\f") {
      index += 1;
      continue;
    }
    if (ch === "-" && sql[index + 1] === "-") {
      const next = sql[index + 2];
      if (next == null || next === " " || next === "\t" || next === "\n" || next === "\r") {
        index += 2;
        while (index < sql.length && sql[index] !== "\n") index += 1;
        continue;
      }
    }
    if (ch === "#") {
      while (index < sql.length && sql[index] !== "\n") index += 1;
      continue;
    }
    if (ch === "/" && sql[index + 1] === "*") {
      index += 2;
      while (index < sql.length && !(sql[index] === "*" && sql[index + 1] === "/")) index += 1;
      index += 2;
      continue;
    }
    if (ch === "'" || ch === '"') {
      const quote = ch;
      index += 1;
      while (index < sql.length) {
        if (sql[index] === "\\") {
          index += 2;
          continue;
        }
        if (sql[index] === quote) {
          if (sql[index + 1] === quote) {
            index += 2;
            continue;
          }
          index += 1;
          break;
        }
        index += 1;
      }
      out.push({ t: "", k: "str" });
      continue;
    }
    if (ch === "`") {
      index += 1;
      let word = "";
      while (index < sql.length) {
        if (sql[index] === "`") {
          if (sql[index + 1] === "`") {
            word += "`";
            index += 2;
            continue;
          }
          index += 1;
          break;
        }
        word += sql[index] ?? "";
        index += 1;
      }
      out.push({ t: word, k: "id" });
      continue;
    }
    if (/[A-Za-z0-9_$@]/.test(ch)) {
      let word = "";
      while (index < sql.length && /[A-Za-z0-9_$@]/.test(sql[index] ?? "")) {
        word += sql[index] ?? "";
        index += 1;
      }
      out.push({ t: word, k: "id" });
      continue;
    }
    if (ch === "*") {
      out.push({ t: "*", k: "star" });
      index += 1;
      continue;
    }
    out.push({ t: ch, k: "sym" });
    index += 1;
  }
  return out;
}
