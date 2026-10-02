import { describe, expect, test } from "bun:test";

import { COMPARISONS, tagline } from "../src/lib/pitch.js";

describe("site pitch", () => {
  test("tagline is stable", () => {
    expect(tagline()).toContain("WinDirStat");
  });

  test("scanscan is the first comparison row", () => {
    expect(COMPARISONS[0]?.tool).toBe("scanscan");
    expect(COMPARISONS[0]?.web).toBe("yes");
  });
});
