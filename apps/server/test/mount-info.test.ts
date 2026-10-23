import { describe, expect, test } from "bun:test";

import { mountEntryFor, parseMounts } from "../src/mount-info.js";

describe("parseMounts", () => {
  test("reads source, mountpoint and filesystem type", () => {
    const entries = parseMounts("/dev/sda1 / ext4 rw,relatime 0 0");
    expect(entries).toEqual([{ path: "/", source: "/dev/sda1", fstype: "ext4" }]);
  });

  test("unescapes octal sequences", () => {
    const entries = parseMounts("/dev/disk/by-label/My\\040Disk /mnt/my\\040disk xfs rw 0 0");
    expect(entries[0]?.source).toBe("/dev/disk/by-label/My Disk");
    expect(entries[0]?.path).toBe("/mnt/my disk");
  });

  test("skips short lines", () => {
    expect(parseMounts("proc /proc")).toEqual([]);
  });
});

describe("mountEntryFor", () => {
  const entries = parseMounts(
    [
      "overlay / overlay rw 0 0",
      "/dev/sda1 /host ext4 rw 0 0",
      "/dev/sdb1 /host/mnt/data xfs rw 0 0",
    ].join("\n"),
  );

  test("picks the longest matching mountpoint", () => {
    expect(mountEntryFor("/host/mnt/data/backup", entries)?.fstype).toBe("xfs");
    expect(mountEntryFor("/host/var", entries)?.fstype).toBe("ext4");
  });

  test("does not match a sibling with the same prefix", () => {
    expect(mountEntryFor("/host2", entries)?.fstype).toBe("overlay");
  });

  test("root matches everything", () => {
    expect(mountEntryFor("/anything", entries)?.path).toBe("/");
  });
});
