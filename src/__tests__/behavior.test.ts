import { describe, expect, it } from "vitest";
import { badgeTone } from "../lib/badge";
import { isDirty, selectedText } from "../lib/dirty";
import { executionSummary, formatElapsed } from "../lib/footer";
import { panes } from "../lib/layout";
import { columnKind, completionSchema, type SchemaSnapshot } from "../lib/schema";

describe("dirty and selection", () => {
  it("treats unlinked text as dirty and linked text against the saved copy", () => {
    expect(isDirty({ sql: "", filePath: null, savedSql: null })).toBe(false);
    expect(isDirty({ sql: "select 1", filePath: null, savedSql: null })).toBe(true);
    expect(isDirty({ sql: "select 1", filePath: "/a.sql", savedSql: "select 1" })).toBe(false);
    expect(isDirty({ sql: "select 2", filePath: "/a.sql", savedSql: "select 1" })).toBe(true);
  });

  it("reads the selected slice", () => {
    expect(selectedText("select 1", 0, 6)).toBe("select");
    expect(selectedText("select 1", 3, 3)).toBe("");
  });
});

describe("footer", () => {
  it("summarizes success and the first failure", () => {
    expect(executionSummary(3, 3, 0)).toBe("3 statements executed");
    expect(executionSummary(1, 1, 0)).toBe("1 statement executed");
    expect(executionSummary(3, 2, 1)).toBe("2 of 3 statements executed · 1 failed");
  });

  it("formats elapsed time", () => {
    expect(formatElapsed(12)).toBe("12 ms");
    expect(formatElapsed(1500)).toBe("1.50 s");
  });
});

describe("badges and layout", () => {
  it("colors known names and honors an explicit color", () => {
    expect(badgeTone("prod", null)).toBe("red");
    expect(badgeTone("dev", null)).toBe("green");
    expect(badgeTone("preview", null)).toBe("blue");
    expect(badgeTone("test", null)).toBe("orange");
    expect(badgeTone("prod", "teal")).toBe("teal");
  });

  it("shows the panes each chiclet asks for", () => {
    expect(panes("all")).toEqual({ schema: true, editor: true, results: true });
    expect(panes("editor-results").schema).toBe(false);
    expect(panes("editor").results).toBe(false);
    expect(panes("results").editor).toBe(false);
  });
});

describe("schema", () => {
  const column = (patch: Partial<Parameters<typeof columnKind>[0]>) =>
    columnKind({
      name: "id",
      dataType: "int",
      columnType: "int",
      columnKey: "",
      foreignKey: false,
      ...patch,
    });

  it("picks an icon kind and lets a primary key win", () => {
    expect(column({ columnKey: "PRI", foreignKey: true })).toBe("pk");
    expect(column({ foreignKey: true, dataType: "int" })).toBe("fk");
    expect(column({ dataType: "varchar", columnType: "varchar(40)" })).toBe("text");
    expect(column({ dataType: "datetime", columnType: "datetime" })).toBe("datetime");
    expect(column({ dataType: "tinyint", columnType: "tinyint(1)" })).toBe("enum");
    expect(column({ dataType: "decimal", columnType: "decimal(10,2)" })).toBe("numeric");
  });

  it("lifts the default schema's tables to the top level", () => {
    const snapshot: SchemaSnapshot = {
      lowerCaseTableNames: 0,
      schemas: [
        {
          name: "fleet",
          tables: [{ name: "TRIP", columns: [{ name: "id", dataType: "int", columnType: "int", columnKey: "PRI", foreignKey: false }] }],
        },
      ],
    };
    const schema = completionSchema(snapshot, "fleet");
    expect(schema.TRIP).toEqual(["id"]);
    expect((schema.fleet as { TRIP: string[] }).TRIP).toEqual(["id"]);
  });
});
