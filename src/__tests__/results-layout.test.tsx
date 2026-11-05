/**
 * @vitest-environment jsdom
 */
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { makeTab, type ResultSection } from "../app/types";
import { fitColumnWidth, jsonPreview } from "../lib/cells";
import { resultMin, sashSize } from "../lib/layout";
import { columnKeys, columnWidths, Results } from "../components/Results";
import { PaneSplit } from "../components/PaneSplit";
import { Footer } from "../components/Footer";
import { SettingsModal } from "../components/SettingsModal";
import { Toolbar } from "../components/Toolbar";

const previousObserver = globalThis.ResizeObserver;
const previousPointer = globalThis.PointerEvent;

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 800 });
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 800 });
  if (!HTMLElement.prototype.setPointerCapture) HTMLElement.prototype.setPointerCapture = () => undefined;
  if (!HTMLElement.prototype.releasePointerCapture) HTMLElement.prototype.releasePointerCapture = () => undefined;
  if (typeof globalThis.PointerEvent === "undefined") {
    class PointerEventPolyfill extends MouseEvent {
      pointerId: number;
      constructor(type: string, props: MouseEventInit & { pointerId?: number } = {}) {
        super(type, props);
        this.pointerId = props.pointerId ?? 1;
      }
    }
    globalThis.PointerEvent = PointerEventPolyfill as unknown as typeof PointerEvent;
  }
  globalThis.ResizeObserver = class {
    constructor(private readonly callback: ResizeObserverCallback) {}
    observe(target: Element) {
      const rect = { width: 800, height: 800, top: 0, left: 0, bottom: 800, right: 800, x: 0, y: 0, toJSON: () => ({}) };
      this.callback([{ target, contentRect: rect } as ResizeObserverEntry], this as unknown as ResizeObserver);
    }
    unobserve() {}
    disconnect() {}
  } as typeof ResizeObserver;
});

afterAll(() => {
  if (previousObserver) globalThis.ResizeObserver = previousObserver;
  else Reflect.deleteProperty(globalThis, "ResizeObserver");
  if (previousPointer) globalThis.PointerEvent = previousPointer;
  else Reflect.deleteProperty(globalThis, "PointerEvent");
});

const section: ResultSection = {
  index: 0,
  sql: "select name",
  columns: [{ name: "name", typeName: "varchar", numeric: false }],
  rows: [["Ada"], [null]],
  rowsAffected: null,
  rowCount: 2,
  error: null,
  elapsedMs: 4,
  done: true,
};

function paneHeights(container: HTMLElement): string[] {
  return [...container.querySelectorAll<HTMLElement>(".split-pane-pane")].map((pane) => pane.style.height);
}

describe("results and layout buttons", () => {
  it("shares result column width and keeps a dragged column", () => {
    expect(columnWidths(["a", "b"], {}, 240)).toEqual([120, 120]);
    expect(columnWidths(["a", "b"], { a: 180 }, 240)).toEqual([180, 72]);
    expect(columnWidths(columnKeys(["id", "id"]), { id: 180 }, 240)).toEqual([180, 72]);
  });
  it("leaves null result values blank and fills one result set", () => {
    const { container } = render(<Results sections={[section]} restored={false} />);
    expect(screen.getByText("Ada")).toBeTruthy();
    expect(screen.queryByText("null")).toBeNull();
    expect(container.querySelector(".stack.solo")).toBeTruthy();
    expect(container.querySelector(".split-pane")).toBeNull();
  });

  it("keeps several result sets stacked", () => {
    const sections = [section, { ...section, index: 1 }, { ...section, index: 2 }];
    const { container } = render(<Results sections={sections} restored={false} />);
    expect(container.querySelector(".stack.solo")).toBeNull();
    const split = container.querySelector<HTMLElement>(".stack.many .split-pane");
    expect(split).toBeTruthy();
    expect(split?.style.minHeight).toBe(`${sections.length * resultMin + (sections.length - 1) * sashSize}px`);
    expect(split?.style.overflow).toBe("visible");
    const panes = [...container.querySelectorAll<HTMLElement>(".split-pane-pane")];
    expect(panes).toHaveLength(3);
    expect(panes.every((pane) => pane.style.overflow === "visible")).toBe(true);
    expect(container.querySelectorAll(".sec")).toHaveLength(3);
    expect(paneHeights(container)).toEqual(["260px", "260px", "260px"]);
    const splits = within(container).getAllByRole("separator");
    expect(splits.map((node) => node.getAttribute("aria-label"))).toEqual(["horizontal divider 1", "horizontal divider 2"]);
    expect(splits.every((node) => node.classList.contains("split-pane-divider") && node.classList.contains("split") && node.classList.contains("y") && node.getAttribute("aria-orientation") === "horizontal")).toBe(true);
    expect(splits.map((node) => (node as HTMLElement).style.height)).toEqual([`${sashSize}px`, `${sashSize}px`]);
  });

  it("keeps dragged result pane sizes when a result set goes away", async () => {
    const three = [section, { ...section, index: 1 }, { ...section, index: 2 }];
    const { container, rerender } = render(<Results sections={three} restored={false} />);
    const divider = within(container).getAllByRole("separator")[0];
    await act(async () => {
      fireEvent.pointerDown(divider, { button: 0, clientY: 100, pointerId: 1, pointerType: "mouse" });
    });
    await act(async () => {
      fireEvent.pointerMove(document, { clientY: 150, pointerId: 1, pointerType: "mouse" });
      await new Promise((resolve) => requestAnimationFrame(() => resolve(undefined)));
    });
    expect(paneHeights(container)).toEqual(["310px", "210px", "260px"]);
    rerender(<Results sections={[section, { ...section, index: 1 }]} restored={false} />);
    expect(paneHeights(container)).toEqual(["310px", "210px"]);
  });

  it("resizes from any part of the pane splitter", () => {
    const onSize = vi.fn();
    render(<PaneSplit axis="y" label="Resize editor" size={() => 200} onSize={onSize} min={140} max={() => 600} />);
    const handle = screen.getByRole("separator", { name: "Resize editor" });
    expect((handle as HTMLElement).style.height).toBe(`${sashSize}px`);
    fireEvent.pointerDown(handle, { button: 0, clientY: 40, pointerId: 1 });
    fireEvent.pointerMove(handle, { clientY: 90, pointerId: 1 });
    expect(onSize).toHaveBeenCalledWith(250);
    fireEvent.pointerUp(handle, { clientY: 90, pointerId: 1 });
  });

  it("hides the database user in the footer", () => {
    render(
      <Footer
        tab={makeTab(null)}
        connection={{ name: "dev", host: "db.example", port: 3306, user: "secret-user", ssl: false, explain_lint: false }}
        onSettings={() => undefined}
      />,
    );
    expect(screen.getByText("dev · db.example:3306")).toBeTruthy();
    expect(screen.queryByText(/secret-user/)).toBeNull();
    expect(screen.getByRole("button", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Light" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Dark" })).toBeNull();
  });

  it("applies a theme from the settings list without a save button", () => {
    const chosen: string[] = [];
    render(<SettingsModal theme="light" onTheme={(id) => chosen.push(id)} onClose={() => undefined} />);
    fireEvent.click(screen.getByRole("option", { name: "One Dark Pro Night Flat" }));
    expect(chosen).toEqual(["one-dark-pro-night-flat"]);
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Save" })).toBeNull();
  });

  it("colors column types and toggles columns from the alias menu", () => {
    const joined: ResultSection = {
      index: 0,
      sql: "select d.id, d.name, t.status from driver d join trip t on t.driver_id = d.id",
      columns: [
        { name: "id", typeName: "INT", numeric: true },
        { name: "name", typeName: "VARCHAR", numeric: false },
        { name: "status", typeName: "ENUM", numeric: false },
      ],
      rows: [["1", "Ada", "active"]],
      rowsAffected: null,
      rowCount: 1,
      error: null,
      elapsedMs: 4,
      done: true,
    };
    const { container } = render(<Results sections={[joined]} restored={false} />);
    const view = within(container);
    const bar = container.querySelector(".reshead") as HTMLElement;
    expect(within(bar).getByRole("button", { name: "Types" }).getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector(".ht.tone-num")).toBeNull();
    expect(container.querySelector(".ht")?.textContent).toBe("INT");
    expect(container.querySelector(".cell.tone-num")?.textContent).toBe("1");
    expect(container.querySelector(".cell.tone-text")?.textContent).toBe("Ada");
    expect(container.querySelector(".cell.tone-enum")?.textContent).toBe("active");
    fireEvent.click(within(bar).getByRole("button", { name: "Types" }));
    expect(container.querySelector(".ht")).toBeNull();
    expect(container.querySelector(".grid.plain")).toBeTruthy();
    expect(view.getByText("Ada")).toBeTruthy();
    fireEvent.click(within(bar).getByRole("button", { name: "Types" }));
    expect(container.querySelector(".ht")?.textContent).toBe("INT");
    fireEvent.click(view.getByRole("button", { name: "Columns" }));
    expect(view.getByRole("button", { name: "Select all" }).className).toContain("on");
    expect(view.getByRole("group", { name: "d" })).toBeTruthy();
    expect(view.getByRole("group", { name: "t" })).toBeTruthy();
    fireEvent.click(view.getByRole("checkbox", { name: /status/ }));
    expect(view.queryByText("active")).toBeNull();
    expect(view.getByText("Ada")).toBeTruthy();
    fireEvent.click(view.getByRole("button", { name: "Unselect all" }));
    expect(view.getByText("No columns shown")).toBeTruthy();
    expect(view.queryByText("Ada")).toBeNull();
    fireEvent.click(view.getByRole("button", { name: "Select all" }));
    expect(view.getByText("Ada")).toBeTruthy();
    expect(view.getByText("active")).toBeTruthy();
  });

  it("copies a cell and views json from the cell menu", () => {
    const raw = `{"name":"Ada","note":"${"y".repeat(80)}"}`;
    const preview = jsonPreview(raw);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    const jsonSection: ResultSection = {
      ...section,
      sql: "select meta, name",
      columns: [
        { name: "meta", typeName: "JSON", numeric: false },
        { name: "name", typeName: "VARCHAR", numeric: false },
      ],
      rows: [[raw, "Ada"]],
    };
    const { container } = render(<Results sections={[jsonSection]} restored={false} />);
    const view = within(container);
    expect(view.getByText(preview)).toBeTruthy();
    expect(view.queryByText(raw)).toBeNull();
    fireEvent.click(view.getByText("Ada"));
    expect(view.getByRole("menuitem", { name: "Copy" })).toBeTruthy();
    expect(view.queryByRole("menuitem", { name: "View" })).toBeNull();
    fireEvent.click(view.getByRole("menuitem", { name: "Copy" }));
    expect(writeText).toHaveBeenCalledWith("Ada");
    fireEvent.click(view.getByText(preview));
    fireEvent.click(view.getByRole("menuitem", { name: "Copy" }));
    expect(writeText).toHaveBeenCalledWith(raw);
    fireEvent.click(view.getByText(preview));
    fireEvent.click(view.getByRole("menuitem", { name: "View" }));
    const dialog = view.getByRole("dialog", { name: "meta" });
    expect(dialog.textContent).toContain('"name": "Ada"');
    fireEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(view.queryByRole("dialog", { name: "meta" })).toBeNull();
  });

  it("widens a column from the header and narrows it back", async () => {
    const note = "n".repeat(40);
    const wide: ResultSection = {
      ...section,
      sql: "select id, note",
      columns: [
        { name: "id", typeName: "INT", numeric: true },
        { name: "note", typeName: "TEXT", numeric: false },
      ],
      rows: [["1", note]],
    };
    const { container } = render(<Results sections={[wide]} restored={false} />);
    const header = () => [...container.querySelectorAll(".cell.head")].find((node) => node.querySelector(".hn")?.textContent === "note") as HTMLElement;
    const width = () => Number.parseFloat(header().style.width);
    const fitted = fitColumnWidth("note", "TEXT", [note], true);
    expect(width()).toBe(72);
    fireEvent.doubleClick(header());
    expect(width()).toBe(fitted);
    fireEvent.click(header());
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 260));
    });
    fireEvent.click(screen.getByRole("menuitem", { name: "Narrow" }));
    expect(width()).toBe(72);
    fireEvent.click(header());
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 260));
    });
    expect(screen.queryByRole("menuitem", { name: "Narrow" })).toBeNull();
    fireEvent.click(screen.getByRole("menuitem", { name: "Widen" }));
    expect(width()).toBe(fitted);
  });

  it("shows layout choices as icons", () => {
    render(<Toolbar tab={makeTab(null)} />);
    expect(screen.getByRole("button", { name: "All" }).textContent).toBe("");
    expect(screen.getByRole("button", { name: "Results below the editor" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Results below the editor, schema hidden" })).toBeTruthy();
    expect(screen.queryByText("Editor + Results")).toBeNull();
  });
});
