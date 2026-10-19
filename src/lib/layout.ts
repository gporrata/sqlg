export type LayoutMode = "all" | "editor-results" | "below" | "below-only" | "editor" | "results";

export const LAYOUTS: { id: LayoutMode; label: string }[] = [
  { id: "all", label: "All" },
  { id: "editor-results", label: "Editor + Results" },
  { id: "below", label: "Results below the editor" },
  { id: "below-only", label: "Results below the editor, schema hidden" },
  { id: "editor", label: "Editor only" },
  { id: "results", label: "Results only" },
];

export function layoutMode(value: string | undefined | null): LayoutMode {
  return LAYOUTS.some((item) => item.id === value) ? (value as LayoutMode) : "all";
}

export function panes(layout: LayoutMode): { schema: boolean; editor: boolean; results: boolean; stack: boolean } {
  return {
    schema: layout === "all" || layout === "below",
    editor: layout !== "results",
    results: layout !== "editor",
    stack: layout === "below" || layout === "below-only",
  };
}

export function clampPane(size: number, min: number, max: number): number {
  const upper = Math.max(min, max);
  return Math.min(upper, Math.max(min, size));
}

export const paneDefaults = { schema: 250, editor: 520, editorHeight: 320 };
export const paneMin = { schema: 160, editor: 220, editorHeight: 140 };
export const columnMin = 72;

export function paneSize(value: number | null | undefined, fallback: number, min: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  return Math.min(4000, Math.max(min, Math.round(value)));
}

export function savedColumnWidths(value: Record<string, number> | null | undefined): Record<string, number> {
  if (!value) return {};
  const next: Record<string, number> = {};
  for (const [name, width] of Object.entries(value)) {
    if (typeof width === "number" && Number.isFinite(width)) next[name] = Math.max(columnMin, Math.round(width));
  }
  return next;
}
