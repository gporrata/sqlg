/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from "vitest";
import { shouldDragTitle } from "../lib/windowChrome";

describe("title drag", () => {
  it("drags empty titlebar space and leaves tabs alone", () => {
    document.body.innerHTML = `
      <div class="titlebar">
        <div class="tab" role="tab"><span class="pill">dev</span><button class="x">×</button></div>
        <button class="plus">+</button>
      </div>
    `;
    const titlebar = document.querySelector(".titlebar");
    const tab = document.querySelector(".tab");
    const pill = document.querySelector(".pill");
    const close = document.querySelector(".x");
    expect(shouldDragTitle(titlebar)).toBe(true);
    expect(shouldDragTitle(tab)).toBe(true);
    expect(shouldDragTitle(pill)).toBe(false);
    expect(shouldDragTitle(close)).toBe(false);
    expect(shouldDragTitle(document.querySelector(".plus"))).toBe(false);
  });
});
