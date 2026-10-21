import { describe, expect, it } from "vitest";
import { cellText, columnSizeAction, fitColumnWidth, isJsonType, jsonPreview, prettyJson } from "../lib/cells";

describe("result cell text", () => {
  it("shows a short piece of a json value", () => {
    const raw = `{"name":"Ada","note":"${"x".repeat(80)}"}`;
    expect(isJsonType("JSON")).toBe(true);
    expect(isJsonType("VARCHAR")).toBe(false);
    const preview = jsonPreview(raw);
    expect(preview.endsWith("…")).toBe(true);
    expect(preview.length).toBeLessThan(raw.length);
    expect(cellText("JSON", raw)).toBe(preview);
    expect(cellText("JSON", null)).toBe("");
    expect(cellText("VARCHAR", raw)).toBe(raw);
  });

  it("pretty prints a json object and leaves other text alone", () => {
    expect(prettyJson('{"name":"Ada","n":1}')).toBe('{\n  "name": "Ada",\n  "n": 1\n}');
    expect(prettyJson(null)).toBe("null");
    expect(prettyJson("not json")).toBe("not json");
  });

  it("sizes a column to its widest value and toggles widen with narrow", () => {
    const narrow = fitColumnWidth("id", "INT", ["1"], true);
    const wide = fitColumnWidth("note", "TEXT", ["x".repeat(40)], true);
    expect(wide).toBeGreaterThan(narrow);
    expect(columnSizeAction(false, narrow, wide)).toBe("widen");
    expect(columnSizeAction(true, wide, wide)).toBe("narrow");
    expect(columnSizeAction(true, narrow, wide)).toBe("widen");
  });
});
