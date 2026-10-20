/**
 * @vitest-environment jsdom
 */
import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { SchemaTree } from "../components/SchemaTree";
import type { SchemaSnapshot } from "../lib/schema";

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, "offsetHeight", { configurable: true, get: () => 800 });
  Object.defineProperty(HTMLElement.prototype, "offsetWidth", { configurable: true, get: () => 250 });
});

const snapshot: SchemaSnapshot = {
  lowerCaseTableNames: 0,
  schemas: [
    {
      name: "book",
      tables: [
        {
          name: "trip",
          columns: [
            { name: "id", dataType: "int", columnType: "int", columnKey: "PRI", foreignKey: false },
            { name: "notes", dataType: "varchar", columnType: "varchar(40)", columnKey: "", foreignKey: false },
          ],
        },
        { name: "driver", columns: [] },
      ],
    },
    { name: "analytics", tables: [{ name: "event", columns: [] }] },
  ],
};

describe("schema tree", () => {
  it("shows schema names and the default schema's tables", () => {
    render(<SchemaTree schema={snapshot} defaultSchema="book" onInsert={() => undefined} />);
    expect(screen.getByText("book")).toBeTruthy();
    expect(screen.getByText("trip")).toBeTruthy();
    expect(screen.getByText("driver")).toBeTruthy();
    expect(screen.getByText("analytics")).toBeTruthy();
    expect(screen.queryByText("event")).toBeNull();
  });

  it("colors a column type label", () => {
    const { container } = render(<SchemaTree schema={snapshot} defaultSchema="book" onInsert={() => undefined} />);
    const view = within(container);
    const trip = view.getByText("trip").closest(".tn");
    fireEvent.click(trip?.querySelector("button") as HTMLButtonElement);
    expect(view.getByText("int").className).toContain("tone-num");
    expect(view.getByText("varchar(40)").className).toContain("tone-text");
    expect(container.querySelector(".gl.tone-text")?.textContent).toBe("Aa");
  });

  it("shows a load failure instead of the empty prompt", () => {
    render(<SchemaTree schema={undefined} phase={{ kind: "error", message: "keychain: keychain access was canceled" }} onRetry={() => undefined} onInsert={() => undefined} />);
    expect(screen.getByText(/keychain access was canceled/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
    expect(screen.queryByText("Connect to load schemas")).toBeNull();
  });
});
