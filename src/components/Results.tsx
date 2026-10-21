import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { snippet } from "../lib/titles";
import { formatElapsed } from "../lib/footer";
import { columnGroups } from "../lib/columns";
import { cellText, columnSizeAction, fitColumnWidth, isJsonType, prettyJson } from "../lib/cells";
import { columnMin } from "../lib/layout";
import { typeTone, type SchemaSnapshot } from "../lib/schema";
import type { ColumnInfo, ResultSection } from "../app/types";

const headerMenuDelay = 220;

type Pop =
  | { kind: "cell"; name: string; value: string | null; json: boolean; x: number; y: number }
  | { kind: "header"; name: string; x: number; y: number };

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
  onColumnWidth?: (name: string, width: number | null) => void;
  schema?: SchemaSnapshot;
  defaultSchema?: string | null;
}) {
  const [showTypes, setShowTypes] = useState(true);
  const bar = <ResultsBar showTypes={showTypes} onToggle={() => setShowTypes((on) => !on)} />;
  if (sections.length === 0) {
    return (
      <section className="right">
        {bar}
        <p className="empty">{restored ? "Run to see results" : "Run a query to see results"}</p>
      </section>
    );
  }
  return (
    <section className="right">
      {bar}
      <div className={sections.length === 1 ? "stack solo" : "stack"}>
        {sections.map((section) => (
          <ResultBlock
            key={section.index}
            section={section}
            columnWidths={savedWidths}
            onColumnWidth={onColumnWidth}
            schema={schema}
            defaultSchema={defaultSchema}
            showTypes={showTypes}
          />
        ))}
      </div>
    </section>
  );
}

function ResultsBar({ showTypes, onToggle }: { showTypes: boolean; onToggle: () => void }) {
  return (
    <div className="reshead">
      <b>Results</b>
      <button type="button" className={showTypes ? "types on" : "types"} aria-pressed={showTypes} onClick={onToggle}>
        Types
      </button>
    </div>
  );
}

function ResultBlock({
  section,
  columnWidths: savedWidths,
  onColumnWidth,
  schema,
  defaultSchema,
  showTypes,
}: {
  section: ResultSection;
  columnWidths: Record<string, number>;
  onColumnWidth?: (name: string, width: number | null) => void;
  schema?: SchemaSnapshot;
  defaultSchema?: string | null;
  showTypes: boolean;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const menuTimer = useRef<number | null>(null);
  const [gridWidth, setGridWidth] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [localWidths, setLocalWidths] = useState<Record<string, number>>({});
  const [menu, setMenu] = useState(false);
  const [pop, setPop] = useState<Pop | null>(null);
  const [jsonView, setJsonView] = useState<{ name: string; value: string | null } | null>(null);
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
  useEffect(() => {
    const clear = () => {
      if (menuTimer.current == null) return;
      window.clearTimeout(menuTimer.current);
      menuTimer.current = null;
    };
    document.addEventListener("pointerdown", clear);
    return () => {
      document.removeEventListener("pointerdown", clear);
      clear();
    };
  }, []);
  const failed = section.error != null;
  const names = shown.map((item) => item.column.name);
  const widths = columnWidths(names, overrides, gridWidth);
  const rowWidth = Math.max(gridWidth, widths.reduce((sum, width) => sum + width, 0));
  const resizeColumn = (name: string, event: ReactPointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    if (menuTimer.current != null) {
      window.clearTimeout(menuTimer.current);
      menuTimer.current = null;
    }
    setPop(null);
    const origin = widths[names.indexOf(name)] ?? columnMin;
    const start = event.clientX;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    const move = (pointer: PointerEvent) => {
      const next = Math.max(columnMin, origin + pointer.clientX - start);
      setColumnWidth(name, next);
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
  const setColumnWidth = (name: string, width: number | null) => {
    if (onColumnWidth) onColumnWidth(name, width);
    else {
      setLocalWidths((current) => {
        const next = { ...current };
        if (width == null) delete next[name];
        else next[name] = width;
        return next;
      });
    }
  };
  const valuesFor = (name: string) => {
    const item = shown.find((entry) => entry.column.name === name);
    if (!item) return [];
    return section.rows.map((row) => row?.[item.index] ?? null);
  };
  const widenColumn = (name: string) => {
    const item = shown.find((entry) => entry.column.name === name);
    if (!item) return;
    const fitted = fitColumnWidth(item.column.name, item.column.typeName, valuesFor(name), showTypes);
    const current = widths[names.indexOf(name)] ?? columnMin;
    setColumnWidth(name, Math.max(current, fitted));
  };
  const openHeaderMenu = (name: string, event: ReactMouseEvent<HTMLElement>) => {
    if (event.detail > 1) return;
    const rect = event.currentTarget.getBoundingClientRect();
    if (menuTimer.current != null) window.clearTimeout(menuTimer.current);
    menuTimer.current = window.setTimeout(() => {
      menuTimer.current = null;
      setPop({ kind: "header", name, x: rect.left, y: rect.bottom + 4 });
    }, headerMenuDelay);
  };
  const headerAction = pop?.kind === "header" ? pop.name : "";
  const headerFitted = headerAction ? fitColumnWidth(headerAction, shown.find((entry) => entry.column.name === headerAction)?.column.typeName ?? "", valuesFor(headerAction), showTypes) : 0;
  const headerCurrent = headerAction ? (widths[names.indexOf(headerAction)] ?? columnMin) : 0;
  const headerSize = columnSizeAction(headerAction ? overrides[headerAction] != null : false, headerCurrent, headerFitted);
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
          <div
            className={showTypes ? "grid" : "grid plain"}
            ref={parentRef}
            onScroll={(event) => {
              setScrollLeft(event.currentTarget.scrollLeft);
              setPop(null);
            }}
          >
          <div className="hr" style={{ width: rowWidth }}>
            {shown.map((item, index) => (
              <div
                key={item.index}
                className={item.column.numeric ? "num cell head" : "cell head"}
                style={{ width: widths[index] }}
                onClick={(event) => openHeaderMenu(item.column.name, event)}
                onDoubleClick={(event) => {
                  event.preventDefault();
                  if (menuTimer.current != null) window.clearTimeout(menuTimer.current);
                  setPop(null);
                  widenColumn(item.column.name);
                }}
              >
                <span className="hn">{item.column.name}</span>
                {showTypes && item.column.typeName ? <span className="ht">{item.column.typeName}</span> : null}
              </div>
            ))}
          </div>
          <div style={{ height: virtualizer.getTotalSize(), width: rowWidth, position: "relative" }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = section.rows[item.index];
              return (
                <div key={item.key} className={item.index % 2 ? "vr even" : "vr"} style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, left: 0, width: rowWidth }}>
                  {shown.map((column, index) => {
                    const value = row?.[column.index] ?? null;
                    return (
                      <span
                        key={column.index}
                        className={`cell tone-${typeTone(column.column.typeName)}${column.column.numeric ? " num" : ""}`}
                        style={{ width: widths[index] }}
                        onClick={(event) => {
                          if (menuTimer.current != null) window.clearTimeout(menuTimer.current);
                          const rect = event.currentTarget.getBoundingClientRect();
                          setPop({
                            kind: "cell",
                            name: column.column.name,
                            value,
                            json: isJsonType(column.column.typeName),
                            x: rect.left,
                            y: rect.bottom + 4,
                          });
                        }}
                      >
                        {cellText(column.column.typeName, value)}
                      </span>
                    );
                  })}
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
      {pop?.kind === "cell" ? (
        <CellMenu
          pop={pop}
          onClose={() => setPop(null)}
          onView={() => {
            setJsonView({ name: pop.name, value: pop.value });
            setPop(null);
          }}
        />
      ) : null}
      {pop?.kind === "header" ? (
        <HeaderMenu
          pop={pop}
          action={headerSize}
          onClose={() => setPop(null)}
          onAction={() => {
            if (headerSize === "narrow") setColumnWidth(pop.name, null);
            else widenColumn(pop.name);
            setPop(null);
          }}
        />
      ) : null}
      {jsonView ? <JsonModal name={jsonView.name} value={jsonView.value} onClose={() => setJsonView(null)} /> : null}
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

function menuPoint(x: number, y: number): { left: number; top: number } {
  const width = 160;
  const height = 88;
  const left = Math.max(8, Math.min(x, window.innerWidth - width - 8));
  const top = Math.max(8, Math.min(y, window.innerHeight - height - 8));
  return { left, top };
}

function CellMenu({ pop, onClose, onView }: { pop: Extract<Pop, { kind: "cell" }>; onClose: () => void; onView: () => void }) {
  const rootRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) onClose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", key);
    };
  }, [onClose]);
  const point = menuPoint(pop.x, pop.y);
  return (
    <div ref={rootRef} className="cellpop" role="menu" aria-label={`${pop.name} cell`} style={point}>
      <button
        type="button"
        role="menuitem"
        onClick={() => {
          copyText(pop.value ?? "");
          onClose();
        }}
      >
        Copy
      </button>
      {pop.json ? (
        <button type="button" role="menuitem" onClick={onView}>
          View
        </button>
      ) : null}
    </div>
  );
}

function HeaderMenu({
  pop,
  action,
  onClose,
  onAction,
}: {
  pop: Extract<Pop, { kind: "header" }>;
  action: "widen" | "narrow";
  onClose: () => void;
  onAction: () => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) onClose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("pointerdown", close);
      document.removeEventListener("keydown", key);
    };
  }, [onClose]);
  const point = menuPoint(pop.x, pop.y);
  const label = action === "narrow" ? "Narrow" : "Widen";
  return (
    <div ref={rootRef} className="cellpop" role="menu" aria-label={`${pop.name} column`} style={point}>
      <button type="button" role="menuitem" onClick={onAction}>
        {label}
      </button>
    </div>
  );
}

function JsonModal({ name, value, onClose }: { name: string; value: string | null; onClose: () => void }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="mask float" role="presentation" onMouseDown={onClose}>
      <div className="modal json" role="dialog" aria-label={name} onMouseDown={(event) => event.stopPropagation()}>
        <div className="mt settings-title">
          <span>{name}</span>
          <button className="x" type="button" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <pre className="json-view">{prettyJson(value)}</pre>
      </div>
    </div>
  );
}

function copyText(value: string) {
  const clipboard = navigator.clipboard;
  if (clipboard && typeof clipboard.writeText === "function") {
    void clipboard.writeText(value).catch(() => copyWithTextarea(value));
    return;
  }
  copyWithTextarea(value);
}

function copyWithTextarea(value: string) {
  const area = document.createElement("textarea");
  area.value = value;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.left = "-9999px";
  document.body.append(area);
  area.select();
  document.execCommand("copy");
  area.remove();
}
