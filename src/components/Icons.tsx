import type { LayoutMode } from "../lib/layout";

export function SunIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="2.4" fill="none" stroke="currentColor" strokeWidth="1.3" />
      <path d="M8 1.4v1.8M8 12.8v1.8M1.4 8h1.8M12.8 8h1.8M3.2 3.2l1.3 1.3M11.5 11.5l1.3 1.3M12.8 3.2l-1.3 1.3M4.5 11.5l-1.3 1.3" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  );
}

export function MoonIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M9.4 1.6a5.8 5.8 0 1 0 5 8.4 4.6 4.6 0 0 1-5-8.4z" fill="none" stroke="currentColor" strokeWidth="1.3" />
    </svg>
  );
}

export function Chevron({ open }: { open: boolean }) {
  return (
    <svg className={open ? "chev open" : "chev"} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path d="M3 2l4 3-4 3" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function SchemaIcon() {
  return (
    <svg className="ic" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <ellipse cx="8" cy="4" rx="5" ry="2" fill="none" stroke="currentColor" />
      <path d="M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4" fill="none" stroke="currentColor" />
    </svg>
  );
}

export function TableIcon() {
  return (
    <svg className="ic" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <rect x="2" y="3" width="12" height="10" rx="1" fill="none" stroke="currentColor" />
      <path d="M2 6.5h12M6 6.5V13" stroke="currentColor" />
    </svg>
  );
}

export function KeyIcon() {
  return (
    <svg className="ic gold" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="5" cy="8" r="2.2" fill="none" stroke="currentColor" />
      <path d="M7 8h7M12 8v2M10 8v1.5" stroke="currentColor" strokeLinecap="round" />
    </svg>
  );
}

export function LinkIcon() {
  return (
    <svg className="ic purple" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M6.5 9.5l-1 1a2.2 2.2 0 0 1-3.1-3.1l2-2a2.2 2.2 0 0 1 3.1 0M9.5 6.5l1-1a2.2 2.2 0 0 1 3.1 3.1l-2 2a2.2 2.2 0 0 1-3.1 0" fill="none" stroke="currentColor" strokeLinecap="round" />
    </svg>
  );
}

export function LayoutGlyph({ mode }: { mode: LayoutMode }) {
  if (mode === "below") {
    return (
      <span className="ico split" aria-hidden="true">
        <b className="f" />
        <span>
          <b className="f" />
          <b className="f" />
        </span>
      </span>
    );
  }
  if (mode === "below-only") {
    return (
      <span className="ico rows" aria-hidden="true">
        <b className="f" />
        <b className="f" />
      </span>
    );
  }
  const on = (part: "s" | "e" | "r") => {
    if (mode === "all") return true;
    if (mode === "editor-results") return part !== "s";
    if (mode === "editor") return part === "e";
    return part === "r";
  };
  return (
    <span className="ico cols" aria-hidden="true">
      <b className={on("s") ? "f" : ""} />
      <b className={on("e") ? "f" : ""} />
      <b className={on("r") ? "f" : ""} />
    </span>
  );
}
