import { describe, expect, test } from "vitest";

import { formatAgo, formatBytes, percent } from "./format.js";

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

describe("formatAgo", () => {
  const now = 1_000_000_000_000;

  test("reads a missing timestamp as never", () => {
    expect(formatAgo(0, now)).toBe("never");
  });

  test("formats minutes, hours and days", () => {
    expect(formatAgo(now - 5_000, now)).toBe("just now");
    expect(formatAgo(now - 12 * 60_000, now)).toBe("12m ago");
    expect(formatAgo(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(formatAgo(now - 4 * 86_400_000, now)).toBe("4d ago");
  });
});
