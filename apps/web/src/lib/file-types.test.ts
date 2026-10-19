import { describe, expect, test } from "vitest";

import type { HistogramBucket } from "@scanscan/api-types";
import { FILE_TYPES, groupByFileType, groupOfExtension } from "./file-types.js";

function bucket(key: string, count: number, size: number): HistogramBucket {
  return { key, label: key, count, size };
}

describe("groupOfExtension", () => {
  test("maps a known extension to its group", () => {
    expect(groupOfExtension("jpg")).toBe("photos");
    expect(groupOfExtension("JPG")).toBe("photos");
    expect(groupOfExtension("mkv")).toBe("videos");
    expect(groupOfExtension("mp3")).toBe("audio");
    expect(groupOfExtension("deb")).toBe("packages");
    expect(groupOfExtension("exe")).toBe("executables");
    expect(groupOfExtension("pdf")).toBe("documents");
    expect(groupOfExtension("cace")).toBe("caches");
  });

  test("sends unknown and empty extensions to others", () => {
    expect(groupOfExtension("")).toBe("other");
    expect(groupOfExtension("zzz")).toBe("other");
  });

  test("lists each extension in one group only", () => {
    const seen: string[] = [];
    for (const group of FILE_TYPES) {
      for (const ext of group.extensions) {
        expect(seen.includes(ext)).toBe(false);
        seen.push(ext);
      }
    }
  });
});

describe("groupByFileType", () => {
  test("aggregates size and count per group, largest first", () => {
    const groups = groupByFileType([
      bucket("jpg", 2, 300),
      bucket("pjp", 1, 100),
      bucket("mp4", 1, 900),
      bucket("zzz", 3, 50),
    ]);
    expect(groups.length).toBe(3);
    expect(groups.at(0)?.id).toBe("videos");
    expect(groups.at(0)?.size).toBe(900);
    expect(groups.at(1)?.id).toBe("photos");
    expect(groups.at(1)?.size).toBe(400);
    expect(groups.at(1)?.count).toBe(3);
    expect(groups.at(2)?.id).toBe("other");
  });

  test("conserves the total bytes and drops empty groups", () => {
    const items = [bucket("jpg", 1, 10), bucket("mkv", 1, 20), bucket("", 2, 30)];
    const groups = groupByFileType(items);
    const sum = groups.reduce((acc, group) => acc + group.size, 0);
    expect(sum).toBe(60);
    expect(groups.some((group) => group.count === 0)).toBe(false);
  });

  test("keeps others last even when it holds the most bytes", () => {
    const groups = groupByFileType([bucket("jpg", 1, 10), bucket("zzz", 1, 900)]);
    expect(groups[0]?.id).toBe("photos");
    expect(groups[1]?.id).toBe("other");
  });
});
