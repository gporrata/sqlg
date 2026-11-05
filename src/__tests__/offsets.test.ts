import { describe, expect, it } from "vitest";
import { utf16ToUtf8Offset, utf8ToUtf16Offset } from "../lib/offsets";

describe("editor offsets", () => {
  it("converts between utf-16 cursor units and utf-8 bytes", () => {
    const sql = "select 'héllo'";
    const at = sql.indexOf("é");
    expect(utf16ToUtf8Offset(sql, at)).toBe(at);
    expect(utf16ToUtf8Offset(sql, at + 1)).toBe(at + 2);
    expect(utf8ToUtf16Offset(sql, at + 2)).toBe(at + 1);
    expect(utf16ToUtf8Offset(sql, sql.length)).toBe(new TextEncoder().encode(sql).length);
    expect(utf8ToUtf16Offset(sql, new TextEncoder().encode(sql).length)).toBe(sql.length);
  });
});
