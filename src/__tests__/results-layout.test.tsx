/**
 * @vitest-environment jsdom
 */
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { makeTab, type ResultSection } from "../app/types";
import { fitColumnWidth, jsonPreview } from "../lib/cells";
import { columnWidths, Results } from "../components/Results";
import { Footer } from "../components/Footer";
import { SettingsModal } from "../components/SettingsModal";
import { Toolbar } from "../components/Toolbar";

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 800 });
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 800 });
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

describe("results and layout buttons", () => {
  it("shares result column width and keeps a dragged column", () => {
    expect(columnWidths(["a", "b"], {}, 240)).toEqual([120, 120]);
    expect(columnWidths(["a", "b"], { a: 180 }, 240)).toEqual([180, 72]);
  });
  it("leaves null result values blank and fills one result set", () => {
    const { container } = render(<Results sections={[section]} restored={false} />);
    expect(screen.getByText("Ada")).toBeTruthy();
    expect(screen.queryByText("null")).toBeNull();
    expect(container.querySelector(".stack.solo")).toBeTruthy();
  });

  it("keeps several result sets stacked", () => {
    const { container } = render(<Results sections={[section, { ...section, index: 1 }]} restored={false} />);
    expect(container.querySelector(".stack.solo")).toBeNull();
    expect(container.querySelector(".stack")).toBeTruthy();
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
