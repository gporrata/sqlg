/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { makeTab, type ResultSection } from "../app/types";
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
    expect(container.querySelector(".ht.tone-num")?.textContent).toBe("INT");
    expect(container.querySelector(".ht.tone-text")?.textContent).toBe("VARCHAR");
    expect(container.querySelector(".ht.tone-enum")?.textContent).toBe("ENUM");
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

  it("shows layout choices as icons", () => {
    render(<Toolbar tab={makeTab(null)} />);
    expect(screen.getByRole("button", { name: "All" }).textContent).toBe("");
    expect(screen.getByRole("button", { name: "Results below the editor" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Results below the editor, schema hidden" })).toBeTruthy();
    expect(screen.queryByText("Editor + Results")).toBeNull();
  });
});
