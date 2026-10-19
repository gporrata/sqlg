/**
 * @vitest-environment jsdom
 */
import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { makeTab, type ResultSection } from "../app/types";
import { columnWidths, Results } from "../components/Results";
import { Footer } from "../components/Footer";
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
        theme="light"
        onTheme={() => undefined}
      />,
    );
    expect(screen.getByText("dev · db.example:3306")).toBeTruthy();
    expect(screen.queryByText(/secret-user/)).toBeNull();
  });

  it("shows layout choices as icons", () => {
    render(<Toolbar tab={makeTab(null)} />);
    expect(screen.getByRole("button", { name: "All" }).textContent).toBe("");
    expect(screen.getByRole("button", { name: "Results below the editor" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Results below the editor, schema hidden" })).toBeTruthy();
    expect(screen.queryByText("Editor + Results")).toBeNull();
  });
});
