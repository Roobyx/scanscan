import { describe, expect, test } from "vitest";

import { tooltipStyle } from "./tooltip.js";

describe("tooltipStyle", () => {
  const box = { width: 1000, height: 800 };

  test("places the tooltip below-right of the pointer when there is room", () => {
    expect(tooltipStyle({ x: 300, y: 200, ...box })).toBe("left: 314px; top: 214px");
  });

  test("flips to the left of the pointer near the right edge", () => {
    expect(tooltipStyle({ x: 900, y: 200, ...box })).toBe("right: 114px; top: 214px");
  });

  test("flips above the pointer near the bottom edge", () => {
    expect(tooltipStyle({ x: 300, y: 760, ...box })).toBe("left: 314px; bottom: 54px");
  });

  test("flips both ways in the bottom-right corner", () => {
    expect(tooltipStyle({ x: 980, y: 790, ...box })).toBe("right: 34px; bottom: 24px");
  });

  test("stays on the default side when neither side has room", () => {
    expect(tooltipStyle({ x: 100, y: 50, width: 200, height: 160 })).toBe("left: 114px; top: 64px");
  });

  test("uses a custom gap", () => {
    expect(tooltipStyle({ x: 300, y: 200, ...box, gap: 4 })).toBe("left: 304px; top: 204px");
  });
});
