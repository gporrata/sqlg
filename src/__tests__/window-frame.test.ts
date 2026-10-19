import { describe, expect, it } from "vitest";
import { moveRect, resizeRect } from "../lib/windowFrame";

const origin = { x: 100, y: 80, width: 1440, height: 900 };

describe("window frame", () => {
  it("moves by the pointer delta", () => {
    expect(moveRect(origin, 40, -15)).toEqual({ x: 140, y: 65 });
  });

  it("resizes from each edge and keeps the minimum size", () => {
    expect(resizeRect(origin, "e", -80, 0)).toMatchObject({ x: 100, width: 1360 });
    expect(resizeRect(origin, "s", 0, 50)).toMatchObject({ y: 80, height: 950 });
    expect(resizeRect(origin, "w", -30, 0)).toMatchObject({ x: 70, width: 1470 });
    expect(resizeRect(origin, "n", 0, -20)).toMatchObject({ y: 60, height: 920 });
    expect(resizeRect(origin, "se", 10, 10)).toMatchObject({ x: 100, y: 80, width: 1450, height: 910 });
    expect(resizeRect({ ...origin, width: 1000, height: 700 }, "e", -200, 0).width).toBe(960);
    expect(resizeRect({ ...origin, width: 1000, height: 700 }, "n", 0, 200).height).toBe(640);
  });
});
