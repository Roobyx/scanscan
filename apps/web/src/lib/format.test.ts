import { describe, expect, test } from "vitest";

import { formatBytes, percent } from "./format.js";

describe("formatBytes", () => {
  test("handles zero and negatives", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(-5)).toBe("0 B");
  });

  test("formats binary units", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024 * 3.5)).toBe("3.5 MB");
    expect(formatBytes(1024 ** 4)).toBe("1.0 TB");
  });
});

describe("percent", () => {
  test("clamps and guards divide-by-zero", () => {
    expect(percent(1, 0)).toBe(0);
    expect(percent(50, 200)).toBe(25);
    expect(percent(500, 100)).toBe(100);
  });
});
