export function isDirty(tab: { sql: string; filePath: string | null; savedSql: string | null }): boolean {
  if (tab.filePath) {
    return tab.sql !== (tab.savedSql ?? "");
  }
  return tab.sql.length > 0;
}

export function selectedText(sql: string, anchor: number | null, head: number | null): string {
  if (anchor == null || head == null || anchor === head) {
    return "";
  }
  const from = Math.min(anchor, head);
  const to = Math.max(anchor, head);
  return sql.slice(from, to);
}
