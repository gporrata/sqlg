import { useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { columnKind, type ColumnKind, type SchemaSnapshot } from "../lib/schema";
import { Chevron, KeyIcon, LinkIcon, SchemaIcon, TableIcon } from "./Icons";

type Row = {
  key: string;
  depth: number;
  label: string;
  typeLabel?: string;
  kind: "schema" | "table" | ColumnKind;
  expandable: boolean;
  open: boolean;
};

export function SchemaTree({ schema, onInsert }: { schema: SchemaSnapshot | undefined; onInsert: (text: string) => void }) {
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const parentRef = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => flatten(schema, open), [schema, open]);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24,
    overscan: 12,
  });
  if (!schema) {
    return (
      <aside className="left">
        <div className="h">Schema</div>
        <p className="empty">Connect to load schemas</p>
      </aside>
    );
  }
  return (
    <aside className="left">
      <div className="h">Schema</div>
      <div className="tree" ref={parentRef}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((item) => {
            const row = rows[item.index];
            return (
              <div
                key={row.key}
                className="tn"
                style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, left: 4, right: 4, paddingLeft: 8 + row.depth * 14 }}
                onDoubleClick={() => onInsert(row.label)}
              >
                {row.depth > 0 ? <i className="ig" style={{ left: 10 + (row.depth - 1) * 14 }} /> : null}
                <button
                  className="cw"
                  type="button"
                  aria-label={row.expandable ? (row.open ? "Collapse" : "Expand") : undefined}
                  onClick={() => {
                    if (!row.expandable) return;
                    setOpen((current) => ({ ...current, [row.key]: !row.open }));
                  }}
                >
                  {row.expandable ? <Chevron open={row.open} /> : null}
                </button>
                <KindIcon kind={row.kind} />
                <span className="nm">{row.label}</span>
                {row.typeLabel ? <span className="ty">{row.typeLabel}</span> : null}
              </div>
            );
          })}
        </div>
      </div>
    </aside>
  );
}

function flatten(schema: SchemaSnapshot | undefined, open: Record<string, boolean>): Row[] {
  if (!schema) return [];
  const rows: Row[] = [];
  for (const group of schema.schemas) {
    const groupOpen = open[group.name] ?? false;
    rows.push({ key: group.name, depth: 0, label: group.name, kind: "schema", expandable: true, open: groupOpen });
    if (!groupOpen) continue;
    for (const table of group.tables) {
      const key = `${group.name}.${table.name}`;
      const tableOpen = open[key] ?? false;
      rows.push({ key, depth: 1, label: table.name, kind: "table", expandable: true, open: tableOpen });
      if (!tableOpen) continue;
      for (const column of table.columns) {
        rows.push({
          key: `${key}.${column.name}`,
          depth: 2,
          label: column.name,
          typeLabel: column.columnType || column.dataType,
          kind: columnKind(column),
          expandable: false,
          open: false,
        });
      }
    }
  }
  return rows;
}

function KindIcon({ kind }: { kind: Row["kind"] }) {
  if (kind === "schema") return <SchemaIcon />;
  if (kind === "table") return <TableIcon />;
  if (kind === "pk") return <KeyIcon />;
  if (kind === "fk") return <LinkIcon />;
  if (kind === "numeric") return <span className="gl">#</span>;
  if (kind === "text") return <span className="gl">Aa</span>;
  if (kind === "datetime") return <span className="gl">◷</span>;
  if (kind === "enum") return <span className="gl">◐</span>;
  return <span className="gl">•</span>;
}
