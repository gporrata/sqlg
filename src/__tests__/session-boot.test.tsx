/**
 * @vitest-environment jsdom
 */
import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useApp } from "../app/store";
import { makeTab } from "../app/types";

const gate = vi.hoisted(() => {
  const writes: string[][] = [];
  const snapshots: Array<{
    schemaWidth?: number;
    editorWidth?: number;
    editorHeight?: number;
    tabs: { sql?: string; layout?: string; columnWidths?: Record<string, number> }[];
  }> = [];
  const listeners = new Map<string, (event: { payload?: unknown }) => void>();
  let removed = false;
  let closed = false;
  let resolveSession: (value: unknown) => void = () => undefined;
  const session = new Promise((resolve) => {
    resolveSession = resolve;
  });
  return {
    writes,
    snapshots,
    listeners,
    get removed() {
      return removed;
    },
    set removed(value: boolean) {
      removed = value;
    },
    get closed() {
      return closed;
    },
    set closed(value: boolean) {
      closed = value;
    },
    session,
    resolveSession: (value: unknown) => resolveSession(value),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class Channel {},
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload?: unknown }) => void) => {
    gate.listeners.set(name, handler);
    return () => gate.listeners.delete(name);
  }),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    label: "main",
    scaleFactor: async () => 1,
    outerPosition: async () => ({ x: 240, y: 68 }),
    outerSize: async () => ({ width: 1440, height: 900 }),
    isFullscreen: async () => false,
    setTitle: async () => undefined,
  }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock("../components/Editor", () => ({ QueryEditor: () => null }));
vi.mock("../components/Results", () => ({ Results: () => null }));
vi.mock("../components/SchemaTree", () => ({ SchemaTree: () => null }));

vi.mock("../app/api", () => ({
  api: {
    loadSession: () => gate.session,
    listConnections: async () => [],
    takePending: async () => null,
    upsertWindow: async (snapshot: (typeof gate.snapshots)[number]) => {
      gate.writes.push(snapshot.tabs.map((tab) => tab.sql ?? ""));
      gate.snapshots.push(snapshot);
    },
    removeWindow: async () => {
      gate.removed = true;
    },
    permitClose: async () => {
      gate.closed = true;
    },
    watchFile: async () => undefined,
    loadSchema: async () => ({ lowerCaseTableNames: 0, schemas: [] }),
    ping: async () => true,
  },
}));

import App from "../app/App";

describe("session boot", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["setTimeout", "setInterval", "clearTimeout", "clearInterval"] });
    Object.assign(window, { __TAURI_INTERNALS__: {} });
    const tab = makeTab(null);
    useApp.setState({
      theme: "light",
      label: "main",
      tabs: [tab],
      activeTabId: tab.id,
      connections: [],
      schemas: {},
      status: {},
      connectionsOpen: false,
      closePrompt: null,
      notice: null,
    });
    gate.writes.length = 0;
    gate.snapshots.length = 0;
    gate.listeners.clear();
    gate.removed = false;
    gate.closed = false;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("does not write the empty tab before the saved session loads", async () => {
    render(<App />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(gate.writes).toEqual([]);

    await act(async () => {
      gate.resolveSession({
        theme: "light",
        windows: [
          {
            label: "main",
            activeTab: "kept",
            schemaWidth: 280,
            editorWidth: 640,
            editorHeight: 360,
            tabs: [{ id: "kept", connectionName: "dev", sql: "select 1", dirty: true, layout: "below", columnWidths: { name: 180 } }],
          },
        ],
      });
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });

    expect(gate.writes.length).toBeGreaterThan(0);
    expect(gate.writes.every((sql) => sql.includes("select 1"))).toBe(true);
    const saved = gate.snapshots[gate.snapshots.length - 1];
    expect(saved?.schemaWidth).toBe(280);
    expect(saved?.editorWidth).toBe(640);
    expect(saved?.editorHeight).toBe(360);
    expect(saved?.tabs[0]?.layout).toBe("below");
    expect(saved?.tabs[0]?.columnWidths).toEqual({ name: 180 });

    gate.snapshots.length = 0;
    await act(async () => {
      gate.listeners.get("native-close")?.({ payload: false });
      for (let i = 0; i < 20 && !gate.closed; i += 1) await Promise.resolve();
    });
    expect(useApp.getState().closePrompt).toBeNull();
    expect(gate.removed).toBe(false);
    expect(gate.closed).toBe(true);
    expect(gate.snapshots[gate.snapshots.length - 1]?.tabs.some((tab) => tab.sql === "select 1")).toBe(true);
  });
});
