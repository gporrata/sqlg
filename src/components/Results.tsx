import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { snippet } from "../lib/titles";
import { formatElapsed } from "../lib/footer";
import type { ResultSection } from "../app/types";

export function Results({ sections, restored }: { sections: ResultSection[]; restored: boolean }) {
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
      <div className="stack">
        {sections.map((section) => (
          <ResultBlock key={section.index} section={section} />
        ))}
      </div>
    </section>
  );
}

function ResultBlock({ section }: { section: ResultSection }) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: section.rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24,
    overscan: 16,
  });
  const failed = section.error != null;
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
        <div className="grid" ref={parentRef}>
          <table>
            <thead>
              <tr>
                {section.columns.map((column) => (
                  <th key={column.name}>{column.name}</th>
                ))}
              </tr>
            </thead>
          </table>
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = section.rows[item.index];
              return (
                <div key={item.key} className={item.index % 2 ? "vr even" : "vr"} style={{ position: "absolute", top: 0, transform: `translateY(${item.start}px)`, left: 0, right: 0 }}>
                  {section.columns.map((column, index) => (
                    <span key={column.name} className={column.numeric ? "num cell" : "cell"}>
                      {row?.[index] ?? "null"}
                    </span>
                  ))}
                </div>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
