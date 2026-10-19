import { useEffect, useState, useRef } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { snippet } from "../lib/titles";
import { formatElapsed } from "../lib/footer";
import { columnMin } from "../lib/layout";
import type { ResultSection } from "../app/types";

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
}: {
  sections: ResultSection[];
  restored: boolean;
  columnWidths?: Record<string, number>;
  onColumnWidth?: (name: string, width: number) => void;
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
          <ResultBlock key={section.index} section={section} columnWidths={savedWidths} onColumnWidth={onColumnWidth} />
        ))}
      </div>
    </section>
  );
}

function ResultBlock({
  section,
  columnWidths: savedWidths,
  onColumnWidth,
}: {
  section: ResultSection;
  columnWidths: Record<string, number>;
  onColumnWidth?: (name: string, width: number) => void;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [gridWidth, setGridWidth] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [localWidths, setLocalWidths] = useState<Record<string, number>>({});
  const overrides = onColumnWidth ? savedWidths : localWidths;
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
  }, [section.columns.length, section.error]);
  const failed = section.error != null;
  const names = section.columns.map((column) => column.name);
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
    <div className={failed ? "sec bad" : "sec"}>
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
      </div>
      {section.error ? <div className="errbox">{section.error}</div> : null}
      {!section.error && section.columns.length === 0 && section.rowsAffected != null ? (
        <div className="aff">{section.rowsAffected} rows affected</div>
      ) : null}
      {!section.error && section.columns.length > 0 ? (
        <div className="gridwrap">
          <div className="grid" ref={parentRef} onScroll={(event) => setScrollLeft(event.currentTarget.scrollLeft)}>
          <div className="hr" style={{ width: rowWidth }}>
            {section.columns.map((column, index) => (
              <div key={column.name} className={column.numeric ? "num cell head" : "cell head"} style={{ width: widths[index] }}>
                <span className="hn">{column.name}</span>
              </div>
            ))}
          </div>
          <div style={{ height: virtualizer.getTotalSize(), width: rowWidth, position: "relative" }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = section.rows[item.index];
              return (
                <div key={item.key} className={item.index % 2 ? "vr even" : "vr"} style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, left: 0, width: rowWidth }}>
                  {section.columns.map((column, index) => (
                    <span key={column.name} className={column.numeric ? "num cell" : "cell"} style={{ width: widths[index] }}>
                      {row?.[index] ?? ""}
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
              return <i key={name} className="colsplit" style={{ left }} onPointerDown={(event) => resizeColumn(name, event)} />;
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
