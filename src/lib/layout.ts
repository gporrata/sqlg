export type LayoutMode = "all" | "editor-results" | "editor" | "results";

export const LAYOUTS: { id: LayoutMode; label: string }[] = [
  { id: "all", label: "All" },
  { id: "editor-results", label: "Editor + Results" },
  { id: "editor", label: "Editor only" },
  { id: "results", label: "Results only" },
];

export function panes(layout: LayoutMode): { schema: boolean; editor: boolean; results: boolean } {
  return {
    schema: layout === "all",
    editor: layout !== "results",
    results: layout !== "editor",
  };
}
