import { describe, expect, it } from "vitest";
import { deriveTitle, tabTooltip, truncate } from "../lib/titles";

describe("deriveTitle", () => {
  it("uses the first line comment and truncates", () => {
    const title = deriveTitle("-- Monthly report for the warehouse team\nselect 1");
    expect(title.label.startsWith("Monthly report")).toBe(true);
    expect(title.label.endsWith("…")).toBe(true);
    expect(title.label.length).toBeLessThanOrEqual(31);
    expect(title.tooltip).toBe("Monthly report for the warehouse team");
  });

  it("uses a hash comment and a block comment's first line", () => {
    expect(deriveTitle("# trips\nselect 1").label).toBe("trips");
    expect(deriveTitle("/*\n  fleet status\n  extra\n*/\nselect 1").label).toBe("fleet status");
  });

  it("ignores comments inside strings and version comments", () => {
    expect(deriveTitle("select '/* hidden */' from trip").label).toContain("select");
    expect(deriveTitle("/*!50500 select secret */\nselect 1 as id").tooltip).toContain("select 1 as id");
  });

  it("collapses a comment-free query and leaves an empty editor untitled", () => {
    expect(deriveTitle("select\n  id\nfrom trip").label).toBe("select id from trip");
    expect(deriveTitle("   \n").placeholder).toBe(true);
  });

  it("adds the file path to the tooltip", () => {
    expect(tabTooltip("-- trips\nselect 1", "/tmp/trips.sql")).toBe("trips — /tmp/trips.sql");
  });

  it("counts characters when truncating", () => {
    expect(truncate("éééé", 2)).toBe("éé…");
  });
});
