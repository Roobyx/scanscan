import { describe, expect, test } from "vitest";

import { colorForKey, extensionOf, hashString, usageColor } from "./color.js";

describe("hashString", () => {
  test("is deterministic and unsigned", () => {
    expect(hashString("abc")).toBe(hashString("abc"));
    expect(hashString("")).toBeGreaterThanOrEqual(0);
    expect(hashString("a")).not.toBe(hashString("b"));
  });
});

describe("colorForKey", () => {
  test("returns a stable hsl color per key", () => {
    expect(colorForKey("alpha")).toBe(colorForKey("alpha"));
    expect(colorForKey("alpha")).toMatch(/^hsl\(\d+ 62% 52%\)$/);
  });
});

describe("extensionOf", () => {
  test("extracts the lowercase extension", () => {
    expect(extensionOf("photo.JPG")).toBe("jpg");
    expect(extensionOf("archive.tar.gz")).toBe("gz");
    expect(extensionOf("Makefile")).toBe("");
    expect(extensionOf(".bashrc")).toBe("");
    expect(extensionOf("trailing.")).toBe("");
  });
});

describe("usageColor", () => {
  test("returns exact stop colors at boundaries", () => {
    expect(usageColor(0)).toBe("rgb(0, 128, 0)"); // 0% green
    expect(usageColor(60)).toBe("rgb(0, 128, 0)"); // 60% green (flat)
    expect(usageColor(70)).toBe("rgb(255, 182, 52)"); // 70% yellow
    expect(usageColor(80)).toBe("rgb(255, 182, 52)"); // 80% yellow (flat)
    expect(usageColor(85)).toBe("rgb(191, 0, 0)"); // 85% red
    expect(usageColor(100)).toBe("rgb(128, 0, 0)"); // 100% deep red
  });

  test("interpolates between stops", () => {
    const mid60 = usageColor(50); // between 0% and 60% (both green) -> still green
    expect(mid60).toBe("rgb(0, 128, 0)");

    const mid75 = usageColor(75); // between 70% yellow and 80% yellow -> yellow
    expect(mid75).toBe("rgb(255, 182, 52)");

    const mid82 = usageColor(82); // between 80% yellow and 85% red -> orange-ish
    expect(mid82).toMatch(/^rgb\(2[0-9][0-9], [0-9]{1,3}, [0-9]{1,3}\)$/);
  });

  test("clamps input to [0, 100]", () => {
    expect(usageColor(-10)).toBe(usageColor(0));
    expect(usageColor(200)).toBe(usageColor(100));
  });
});
