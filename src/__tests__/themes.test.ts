/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { THEMES, applyTheme, themeById } from "../app/themes";

function channel(hex: string, index: number): number {
  return parseInt(hex.slice(1 + index * 2, 3 + index * 2), 16);
}

function linear(value: number): number {
  const unit = value / 255;
  return unit <= 0.04045 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
}

function luminance(hex: string): number {
  return 0.2126 * linear(channel(hex, 0)) + 0.7152 * linear(channel(hex, 1)) + 0.0722 * linear(channel(hex, 2));
}

function contrast(foreground: string, background: string): number {
  const lighter = Math.max(luminance(foreground), luminance(background));
  const darker = Math.min(luminance(foreground), luminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

describe("themes", () => {
  it("keeps known theme ids and falls back to light", () => {
    expect(themeById("dark").id).toBe("dark");
    expect(themeById("one-dark-pro-night-flat").name).toBe("One Dark Pro Night Flat");
    expect(themeById("missing").id).toBe("light");
    expect(themeById(undefined).id).toBe("light");
    expect(new Set(THEMES.map((item) => item.id)).size).toBe(THEMES.length);
    // Also update is_light_theme in src-tauri/src/windows.rs.
    expect(THEMES.filter((item) => !item.dark).map((item) => item.id).sort()).toEqual([
      "github-light",
      "light",
      "solarized-light",
    ]);
  });

  it("uses One Dark Pro Night Flat editor colors", () => {
    const night = themeById("one-dark-pro-night-flat");
    expect(night.dark).toBe(true);
    expect(night.vars["--ed-bg"]).toBe("#16191d");
    expect(night.vars["--ed-fg"]).toBe("#abb2bf");
    expect(night.vars["--win"]).toBe("#16191d");
    expect(night.vars["--side"]).toBe("#16191d");
    expect(night.vars["--tool"]).toBe("#1e2227");
    expect(night.vars["--syn-keyword"]).toBe("#c678dd");
    expect(night.vars["--syn-string"]).toBe("#98c379");
    expect(night.vars["--syn-number"]).toBe("#d19a66");
    expect(night.vars["--syn-comment"]).toBe("#5c6370");
    expect(night.vars["--syn-operator"]).toBe("#56b6c2");
    expect(night.vars["--syn-type"]).toBe("#e5c07b");
    expect(night.vars["--caret"] ?? night.vars["--ed-caret"]).toBe("#528bff");
    expect(night.vars["--sel"]).toBe("#677696");
    expect(night.vars["--tip"]).toBe("#1e2227");
    expect(night.vars["--tip-bd"]).toBe("#3e4452");
    expect(night.vars["--tip-err-bd"]).toBe("#c24038");
    expect(night.vars["--tip-warn-bd"]).toBe("#d19a66");
    expect(night.vars["--accent"]).toBe("#4972c1");
  });

  it("keeps selected text, tooltips, and chrome readable", () => {
    const failures: string[] = [];
    for (const item of THEMES) {
      const pair = (name: string, fg: string, bg: string, minimum: number) => {
        const ratio = contrast(item.vars[fg], item.vars[bg]);
        if (ratio < minimum) failures.push(`${item.id} ${name} ${ratio.toFixed(2)}`);
      };
      pair("selection", "--sel-fg", "--sel", 4.5);
      pair("editor", "--ed-fg", "--ed-bg", 4.5);
      pair("window", "--fg", "--win", 4.5);
      pair("tooltip", "--tip-fg", "--tip", 4.5);
      pair("error tooltip", "--tip-err-fg", "--tip-err", 4.5);
      pair("warning tooltip", "--tip-warn-fg", "--tip-warn", 4.5);
      pair("keyword", "--syn-keyword", "--ed-bg", 4.5);
      pair("string", "--syn-string", "--ed-bg", 4.5);
      pair("number", "--syn-number", "--ed-bg", 4.5);
      pair("operator", "--syn-operator", "--ed-bg", 4.5);
      pair("type", "--syn-type", "--ed-bg", 4.5);
      pair("accent", "--accent-fg", "--accent", 4.5);
      pair("error", "--err", "--err-bg", 4.5);
      pair("banner", "--banner-fg", "--banner", 4.5);
      pair("comment", "--syn-comment", "--ed-bg", 2.2);
      for (const tone of ["green", "red", "blue", "orange", "purple", "teal"]) {
        pair(tone, `--pill-${tone}-fg`, `--pill-${tone}-bg`, 4.5);
      }
      for (const tone of ["num", "text", "time", "enum", "other"]) {
        pair(`type ${tone} on side`, `--tone-${tone}`, "--side", 4.5);
        pair(`type ${tone} on tool`, `--tone-${tone}`, "--tool", 4.5);
      }
    }
    expect(failures).toEqual([]);
  });

  it("applies the chosen theme immediately on the document", () => {
    const applied = applyTheme("nord");
    expect(applied.id).toBe("nord");
    expect(document.documentElement.dataset.theme).toBe("nord");
    expect(document.documentElement.dataset.tone).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
    expect(document.documentElement.style.getPropertyValue("--ed-bg")).toBe("#2e3440");
    applyTheme("not-a-theme");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.style.colorScheme).toBe("light");
  });
});
