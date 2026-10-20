import { create } from "zustand";
import { executionSummary } from "../lib/footer";
import { paneDefaults, type LayoutMode } from "../lib/layout";
import type { SchemaSnapshot } from "../lib/schema";
import { emptyFooter, makeTab, type CloseChoice, type Connection, type QueryEvent, type Tab } from "./types";

type ClosePrompt = { tabIds: string[]; resolve: (choice: CloseChoice) => void };

type AppState = {
  theme: string;
  label: string;
  tabs: Tab[];
  activeTabId: string;
  connections: Connection[];
  schemas: Record<string, SchemaSnapshot>;
  status: Record<string, "unknown" | "ok" | "down">;
  connectionsOpen: boolean;
  settingsOpen: boolean;
  closePrompt: ClosePrompt | null;
  notice: string | null;
  schemaWidth: number;
  editorWidth: number;
  editorHeight: number;
  setTheme: (theme: string) => void;
  setLabel: (label: string) => void;
  setConnections: (connections: Connection[]) => void;
  setSchema: (name: string, schema: SchemaSnapshot) => void;
  setStatus: (name: string, status: "unknown" | "ok" | "down") => void;
  setConnectionsOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setNotice: (notice: string | null) => void;
  setSchemaWidth: (schemaWidth: number) => void;
  setEditorWidth: (editorWidth: number) => void;
  setEditorHeight: (editorHeight: number) => void;
  setPaneSizes: (sizes: { schemaWidth: number; editorWidth: number; editorHeight: number }) => void;
  setActive: (id: string) => void;
  updateTab: (id: string, patch: Partial<Tab>) => void;
  replaceTabs: (tabs: Tab[], activeTabId: string) => void;
  addTab: (tab: Tab, activate?: boolean) => void;
  closeTab: (id: string) => void;
  applyQuery: (tabId: string, event: QueryEvent) => void;
  askClose: (tabIds: string[]) => Promise<CloseChoice>;
  answerClose: (choice: CloseChoice) => void;
};

export const useApp = create<AppState>((set, get) => ({
  theme: "light",
  label: "main",
  tabs: [makeTab(null)],
  activeTabId: "",
  connections: [],
  schemas: {},
  status: {},
  connectionsOpen: false,
  settingsOpen: false,
  closePrompt: null,
  notice: null,
  schemaWidth: paneDefaults.schema,
  editorWidth: paneDefaults.editor,
  editorHeight: paneDefaults.editorHeight,
  setTheme: (theme) => set({ theme }),
  setLabel: (label) => set({ label }),
  setConnections: (connections) => set({ connections }),
  setSchema: (name, schema) => set((state) => ({ schemas: { ...state.schemas, [name]: schema } })),
  setStatus: (name, status) => set((state) => ({ status: { ...state.status, [name]: status } })),
  setConnectionsOpen: (connectionsOpen) => set({ connectionsOpen }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setNotice: (notice) => set({ notice }),
  setSchemaWidth: (schemaWidth) => set({ schemaWidth }),
  setEditorWidth: (editorWidth) => set({ editorWidth }),
  setEditorHeight: (editorHeight) => set({ editorHeight }),
  setPaneSizes: (sizes) => set(sizes),
  setActive: (activeTabId) => set({ activeTabId }),
  updateTab: (id, patch) =>
    set((state) => ({
      tabs: state.tabs.map((tab) => (tab.id === id ? { ...tab, ...patch } : tab)),
    })),
  replaceTabs: (tabs, activeTabId) => set({ tabs: tabs.length ? tabs : [makeTab(null)], activeTabId: activeTabId || tabs[0]?.id || "" }),
  addTab: (tab, activate = true) =>
    set((state) => ({
      tabs: [...state.tabs, tab],
      activeTabId: activate ? tab.id : state.activeTabId || tab.id,
    })),
  closeTab: (id) =>
    set((state) => {
      const tabs = state.tabs.filter((tab) => tab.id !== id);
      const next = tabs.length ? tabs : [makeTab(null)];
      const activeTabId = state.activeTabId === id ? next[Math.max(0, next.length - 1)].id : state.activeTabId;
      return { tabs: next, activeTabId };
    }),
  applyQuery: (tabId, event) =>
    set((state) => ({
      tabs: state.tabs.map((tab) => (tab.id === tabId ? reduceQuery(tab, event) : tab)),
    })),
  askClose: (tabIds) =>
    new Promise((resolve) => {
      set({ closePrompt: { tabIds, resolve } });
    }),
  answerClose: (choice) => {
    const prompt = get().closePrompt;
    set({ closePrompt: null });
    prompt?.resolve(choice);
  },
}));

useApp.setState({ activeTabId: useApp.getState().tabs[0].id });

function reduceQuery(tab: Tab, event: QueryEvent): Tab {
  if (event.kind === "begin") {
    const section = {
      index: event.index,
      sql: event.sql,
      columns: [],
      rows: [],
      rowsAffected: null,
      rowCount: 0,
      error: null,
      elapsedMs: null,
      done: false,
    };
    const results = tab.results.filter((item) => item.index !== event.index);
    results.push(section);
    results.sort((a, b) => a.index - b.index);
    return { ...tab, running: true, runId: event.runId, results, footer: { text: "Running…", tone: "run", elapsedMs: null } };
  }
  const results = tab.results.map((section) => {
    if (section.index !== ("index" in event ? event.index : -1)) {
      return section;
    }
    if (event.kind === "columns") {
      return { ...section, columns: event.columns };
    }
    if (event.kind === "rows") {
      return { ...section, rows: section.rows.concat(event.rows), rowCount: section.rowCount + event.rows.length };
    }
    if (event.kind === "statementDone") {
      return {
        ...section,
        rowsAffected: event.rowsAffected,
        rowCount: event.rowCount,
        elapsedMs: event.elapsedMs,
        done: true,
      };
    }
    if (event.kind === "statementError") {
      return { ...section, error: event.message, done: true };
    }
    return section;
  });
  if (event.kind === "finished") {
    const text = event.cancelled ? "Cancelled" : executionSummary(event.total, event.executed, event.failed);
    return {
      ...tab,
      results,
      running: false,
      footer: {
        text,
        tone: event.cancelled ? "fail" : event.failed ? "fail" : "ok",
        elapsedMs: event.elapsedMs,
      },
    };
  }
  return { ...tab, results };
}

export function activeTab(): Tab | undefined {
  const state = useApp.getState();
  return state.tabs.find((tab) => tab.id === state.activeTabId) ?? state.tabs[0];
}

export function layoutOf(tab: Tab | undefined): LayoutMode {
  return tab?.layout ?? "all";
}

export { emptyFooter };
