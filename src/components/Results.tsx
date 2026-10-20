import { useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { snippet } from "../lib/titles";
import { formatElapsed } from "../lib/footer";
import { columnGroups } from "../lib/columns";
import { columnMin } from "../lib/layout";
import { typeTone, type SchemaSnapshot } from "../lib/schema";
import type { ColumnInfo, ResultSection } from "../app/types";

export function columnWidths(names: string[], overrides: Record<string, number>, gridWidth: number): number[] {
  const used = names.reduce((sum, name) => sum + (overrides[name] ?? 0), 0);
  const free = names.filter((name) => overrides[name] == null).length;
  const share = free > 0 ? Math.max(columnMin, (Math.max(gridWidth, 0) - used) / free) : columnMin;
  return names.map((name) => overrides[name] ?? share);
}

export function Results({
  sections,
  restored,
  columnWidths: savedWidths = {},
  onColumnWidth,
  schema,
  defaultSchema,
}: {
  sections: ResultSection[];
  restored: boolean;
  columnWidths?: Record<string, number>;
  onColumnWidth?: (name: string, width: number) => void;
  schema?: SchemaSnapshot;
  defaultSchema?: string | null;
}) {
  if (sections.length === 0) {
    return (
      <section className="right">
        <div className="reshead">
          <b>Results</b>
        </div>
        <p className="empty">{restored ? "Run to see results" : "Run a query to see results"}</p>
      </section>
    );
  }
  return (
    <section className="right">
      <div className="reshead">
        <b>Results</b>
      </div>
      <div className={sections.length === 1 ? "stack solo" : "stack"}>
        {sections.map((section) => (
          <ResultBlock
            key={section.index}
            section={section}
            columnWidths={savedWidths}
            onColumnWidth={onColumnWidth}
            schema={schema}
            defaultSchema={defaultSchema}
          />
        ))}
      </div>
    </section>
  );
}

function ResultBlock({
  section,
  columnWidths: savedWidths,
  onColumnWidth,
  schema,
  defaultSchema,
}: {
  section: ResultSection;
  columnWidths: Record<string, number>;
  onColumnWidth?: (name: string, width: number) => void;
  schema?: SchemaSnapshot;
  defaultSchema?: string | null;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [gridWidth, setGridWidth] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [localWidths, setLocalWidths] = useState<Record<string, number>>({});
  const [menu, setMenu] = useState(false);
  const signature = section.columns.map((column) => `${column.name}\0${column.typeName}`).join("\n");
  const seen = useRef(signature);
  const [hidden, setHidden] = useState<ReadonlySet<number>>(() => new Set());
  if (seen.current !== signature) {
    seen.current = signature;
    setHidden(new Set());
  }
  const overrides = onColumnWidth ? savedWidths : localWidths;
  const shown = section.columns
    .map((column, index) => ({ column, index }))
    .filter((item) => !hidden.has(item.index));
  const virtualizer = useVirtualizer({
    count: section.rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24,
    overscan: 16,
  });
  useEffect(() => {
    const grid = parentRef.current;
    if (!grid) return;
    const measure = () => setGridWidth(grid.clientWidth);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(grid);
    return () => observer.disconnect();
  }, [section.columns.length, section.error, shown.length]);
  const failed = section.error != null;
  const names = shown.map((item) => item.column.name);
  const widths = columnWidths(names, overrides, gridWidth);
  const rowWidth = Math.max(gridWidth, widths.reduce((sum, width) => sum + width, 0));
  const resizeColumn = (name: string, event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const origin = widths[names.indexOf(name)] ?? columnMin;
    const start = event.clientX;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    const move = (pointer: PointerEvent) => {
      const next = Math.max(columnMin, origin + pointer.clientX - start);
      if (onColumnWidth) onColumnWidth(name, next);
      else setLocalWidths((current) => ({ ...current, [name]: next }));
    };
    const end = () => {
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", end);
      target.removeEventListener("pointercancel", end);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", end);
    target.addEventListener("pointercancel", end);
  };
  return (
    <div className={failed ? (menu ? "sec bad menu" : "sec bad") : menu ? "sec menu" : "sec"}>
      <div className="sh">
        <b>{section.index + 1}</b>
        <code>{snippet(section.sql)}</code>
        <span>
          {section.done && !failed
            ? section.columns.length
              ? `${section.rowCount} rows`
              : section.rowsAffected != null
                ? `${section.rowsAffected} rows affected`
                : `${section.rowCount} rows`
            : failed
              ? "failed"
              : "running"}
          {section.elapsedMs != null ? ` · ${formatElapsed(section.elapsedMs)}` : ""}
        </span>
        {!failed && section.columns.length > 0 ? (
          <ColumnPicker
            sql={section.sql}
            columns={section.columns}
            hidden={hidden}
            schema={schema}
            defaultSchema={defaultSchema}
            open={menu}
            onOpen={setMenu}
            onHidden={setHidden}
          />
        ) : null}
      </div>
      {section.error ? <div className="errbox">{section.error}</div> : null}
      {!section.error && section.columns.length === 0 && section.rowsAffected != null ? (
        <div className="aff">{section.rowsAffected} rows affected</div>
      ) : null}
      {!section.error && section.columns.length > 0 && shown.length === 0 ? <p className="empty">No columns shown</p> : null}
      {!section.error && shown.length > 0 ? (
        <div className="gridwrap">
          <div className="grid" ref={parentRef} onScroll={(event) => setScrollLeft(event.currentTarget.scrollLeft)}>
          <div className="hr" style={{ width: rowWidth }}>
            {shown.map((item, index) => (
              <div key={item.index} className={item.column.numeric ? "num cell head" : "cell head"} style={{ width: widths[index] }}>
                <span className="hn">{item.column.name}</span>
                {item.column.typeName ? <span className={`ht tone-${typeTone(item.column.typeName)}`}>{item.column.typeName}</span> : null}
              </div>
            ))}
          </div>
          <div style={{ height: virtualizer.getTotalSize(), width: rowWidth, position: "relative" }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = section.rows[item.index];
              return (
                <div key={item.key} className={item.index % 2 ? "vr even" : "vr"} style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, left: 0, width: rowWidth }}>
                  {shown.map((column, index) => (
                    <span key={column.index} className={column.column.numeric ? "num cell" : "cell"} style={{ width: widths[index] }}>
                      {row?.[column.index] ?? ""}
                    </span>
                  ))}
                </div>
              );
            })}
          </div>
          </div>
          <div className="colrules">
            {names.map((name, index) => {
              const left = widths.slice(0, index + 1).reduce((sum, width) => sum + width, 0) - scrollLeft;
              return <i key={`${shown[index]?.index ?? index}:${name}`} className="colsplit" style={{ left }} onPointerDown={(event) => resizeColumn(name, event)} />;
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}

function ColumnPicker({
  sql,
  columns,
  hidden,
  schema,
  defaultSchema,
  open,
  onOpen,
  onHidden,
}: {
  sql: string;
  columns: ColumnInfo[];
  hidden: ReadonlySet<number>;
  schema?: SchemaSnapshot;
  defaultSchema?: string | null;
  open: boolean;
  onOpen: (open: boolean) => void;
  onHidden: (hidden: ReadonlySet<number>) => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const groups = useMemo(() => columnGroups(sql, columns, schema, defaultSchema), [sql, columns, schema, defaultSchema]);
  useEffect(() => {
    if (!open) return;
    const close = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) onOpen(false);
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onOpen(false);
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", key);
    };
  }, [open, onOpen]);
  const allOn = hidden.size === 0;
  const noneOn = columns.length > 0 && hidden.size === columns.length;
  const toggle = (index: number) => {
    const next = new Set(hidden);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    onHidden(next);
  };
  return (
    <div className="colpick" ref={rootRef}>
      <button type="button" className="sb" aria-haspopup="dialog" aria-expanded={open} onClick={() => onOpen(!open)}>
        Columns
      </button>
      {open ? (
        <div className="colpop" role="dialog" aria-label="Columns">
          <div className="seg" role="group" aria-label="Column selection">
            <button type="button" className={allOn ? "on" : ""} aria-pressed={allOn} onClick={() => onHidden(new Set())}>
              Select all
            </button>
            <button
              type="button"
              className={noneOn ? "on" : ""}
              aria-pressed={noneOn}
              onClick={() => onHidden(new Set(columns.map((_, index) => index)))}
            >
              Unselect all
            </button>
          </div>
          {groups.map((group) => (
            <div key={group.alias} role="group" aria-label={group.alias}>
              <div className="colgroup">{group.alias}</div>
              {group.columns.map((column) => (
                <label key={column.index} className="colopt">
                  <input type="checkbox" checked={!hidden.has(column.index)} onChange={() => toggle(column.index)} />
                  <span className="nm">{column.name}</span>
                  {column.typeName ? <span className={`ty tone-${typeTone(column.typeName)}`}>{column.typeName}</span> : null}
                </label>
              ))}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
