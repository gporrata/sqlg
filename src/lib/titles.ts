export type DerivedTitle = {
  label: string;
  tooltip: string;
  placeholder: boolean;
};

export function deriveTitle(sql: string): DerivedTitle {
  const comment = firstCommentText(sql);
  if (comment != null) {
    const full = collapse(comment);
    if (full) {
      return { label: truncate(full, 30), tooltip: full, placeholder: false };
    }
  }
  const query = collapse(firstStatement(sql));
  if (!query) {
    return { label: "", tooltip: "", placeholder: true };
  }
  return { label: query, tooltip: query, placeholder: false };
}

export function tabTooltip(sql: string, filePath: string | null): string {
  const title = deriveTitle(sql);
  const base = title.placeholder ? "New query" : title.tooltip;
  return filePath ? `${base} — ${filePath}` : base;
}

export function truncate(value: string, maxChars: number): string {
  const chars = Array.from(value);
  if (chars.length <= maxChars) {
    return value;
  }
  return `${chars.slice(0, maxChars).join("")}…`;
}

export function collapse(value: string): string {
  return value.split(/\s+/).filter(Boolean).join(" ");
}

export function snippet(sql: string, maxChars = 72): string {
  return truncate(collapse(sql), maxChars);
}

function firstCommentText(sql: string): string | null {
  let i = 0;
  while (i < sql.length) {
    const ch = sql[i];
    const next = sql[i + 1];
    if (ch === "'" || ch === '"' || ch === "`") {
      i = skipQuote(sql, i, ch);
      continue;
    }
    if (ch === "#") {
      return lineRest(sql, i + 1);
    }
    if (ch === "-" && next === "-" && isLineComment(sql, i + 2)) {
      return lineRest(sql, i + 2);
    }
    if (ch === "/" && next === "*") {
      if (sql[i + 2] === "!") {
        const end = sql.indexOf("*/", i + 3);
        i = end === -1 ? sql.length : end + 2;
        continue;
      }
      const end = sql.indexOf("*/", i + 2);
      const body = sql.slice(i + 2, end === -1 ? sql.length : end);
      const line = body
        .split(/\r?\n/)
        .map((part) => part.trim())
        .find((part) => part.length > 0);
      return line ?? "";
    }
    i += 1;
  }
  return null;
}

function isLineComment(sql: string, at: number): boolean {
  const ch = sql[at];
  return ch === undefined || /\s/.test(ch);
}

function lineRest(sql: string, at: number): string {
  const end = sql.indexOf("\n", at);
  return sql.slice(at, end === -1 ? sql.length : end);
}

function skipQuote(sql: string, start: number, quote: string): number {
  let i = start + 1;
  while (i < sql.length) {
    if (sql[i] === "\\") {
      i += 2;
      continue;
    }
    if (sql[i] === quote) {
      if (sql[i + 1] === quote) {
        i += 2;
        continue;
      }
      return i + 1;
    }
    i += 1;
  }
  return sql.length;
}

export function firstStatement(sql: string): string {
  let i = 0;
  while (i < sql.length) {
    const ch = sql[i];
    const next = sql[i + 1];
    if (ch === "'" || ch === '"' || ch === "`") {
      i = skipQuote(sql, i, ch);
      continue;
    }
    if (ch === "#") {
      const end = sql.indexOf("\n", i);
      i = end === -1 ? sql.length : end + 1;
      continue;
    }
    if (ch === "-" && next === "-" && isLineComment(sql, i + 2)) {
      const end = sql.indexOf("\n", i);
      i = end === -1 ? sql.length : end + 1;
      continue;
    }
    if (ch === "/" && next === "*") {
      const end = sql.indexOf("*/", i + 2);
      i = end === -1 ? sql.length : end + 2;
      continue;
    }
    if (ch === ";") {
      return sql.slice(0, i);
    }
    i += 1;
  }
  return sql;
}
