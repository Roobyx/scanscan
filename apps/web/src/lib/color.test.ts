import { describe, expect, test } from "vitest";

import { colorForExtension, colorForKey, extensionOf, hashString } from "./color.js";

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

describe("colorForExtension", () => {
  test("is stable and falls back for empty extensions", () => {
    expect(colorForExtension("png")).toBe(colorForExtension("png"));
    expect(colorForExtension("")).toBe("#5b6577");
  });
});
