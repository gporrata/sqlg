import { useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { columnKind, typeTone, type ColumnKind, type SchemaSnapshot } from "../lib/schema";
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

export type SchemaPhase = { kind: "loading" } | { kind: "error"; message: string };

export function SchemaTree({
  schema,
  defaultSchema,
  phase,
  onRetry,
  onInsert,
}: {
  schema: SchemaSnapshot | undefined;
  defaultSchema?: string | null;
  phase?: SchemaPhase;
  onRetry?: () => void;
  onInsert: (text: string) => void;
}) {
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const parentRef = useRef<HTMLDivElement>(null);
  const rows = useMemo(() => flatten(schema, open, defaultSchema), [schema, open, defaultSchema]);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24,
    overscan: 12,
    initialRect: { width: 242, height: 640 },
  });
  if (!schema) {
    return (
      <aside className="left">
        <div className="h">Schema</div>
        {phase?.kind === "error" ? (
          <p className="empty">
            {phase.message}
            {onRetry ? (
              <>
                {" "}
                <button type="button" className="btn" onClick={onRetry}>
                  Retry
                </button>
              </>
            ) : null}
          </p>
        ) : (
          <p className="empty">{phase?.kind === "loading" ? "Loading schemas…" : "Connect to load schemas"}</p>
        )}
      </aside>
    );
  }
  const virtualItems = virtualizer.getVirtualItems();
  const painted =
    virtualItems.length > 0
      ? virtualItems.map((item) => ({ key: String(item.key), index: item.index, start: item.start }))
      : rows.map((row, index) => ({ key: row.key, index, start: index * 24 }));
  return (
    <aside className="left">
      <div className="h">Schema</div>
      {rows.length === 0 ? <p className="empty">No tables</p> : null}
      <div className="tree" ref={parentRef}>
        <div style={{ height: Math.max(virtualizer.getTotalSize(), rows.length * 24), position: "relative" }}>
          {painted.map((item) => {
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
                {row.typeLabel ? <span className={`ty tone-${typeTone(row.typeLabel)}`}>{row.typeLabel}</span> : null}
              </div>
            );
          })}
        </div>
      </div>
    </aside>
  );
}

function flatten(schema: SchemaSnapshot | undefined, open: Record<string, boolean>, defaultSchema?: string | null): Row[] {
  if (!schema) return [];
  const rows: Row[] = [];
  for (const group of schema.schemas) {
    const groupOpen = open[group.name] ?? (group.name === defaultSchema || schema.schemas.length === 1);
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
  if (kind === "numeric") return <span className="gl tone-num">#</span>;
  if (kind === "text") return <span className="gl tone-text">Aa</span>;
  if (kind === "datetime") return <span className="gl tone-time">◷</span>;
  if (kind === "enum") return <span className="gl tone-enum">◐</span>;
  return <span className="gl tone-other">•</span>;
}
