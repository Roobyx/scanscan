import { describe, expect, test } from "vitest";

import { isSystemMount, mountLabel, orderMounts } from "./mounts.js";

describe("mount helpers", () => {
  test("recognises the Linux system mounts with or without a trailing slash", () => {
    expect(isSystemMount("/tmp")).toBe(true);
    expect(isSystemMount("/tmp/")).toBe(true);
    expect(isSystemMount("/var/log")).toBe(true);
    expect(isSystemMount("/boot/efi")).toBe(true);
    expect(isSystemMount("/mnt/data")).toBe(false);
    expect(isSystemMount("/")).toBe(false);
  });

  test("labels the host root as the absolute root", () => {
    expect(mountLabel("/")).toBe("Absolute root");
    expect(mountLabel("/mnt/data")).toBe("/mnt/data");
  });

  test("keeps user mounts first and system mounts last without reordering either group", () => {
    const mounts = [
      { path: "/tmp" },
      { path: "/mnt/data" },
      { path: "/" },
      { path: "/var/log" },
      { path: "/boot/efi" },
      { path: "/mnt/backup" },
    ];
    expect(orderMounts(mounts).map((mount) => mount.path)).toEqual([
      "/mnt/data",
      "/",
      "/mnt/backup",
      "/tmp",
      "/var/log",
      "/boot/efi",
    ]);
  });

  test("does not mutate the caller's array", () => {
    const mounts = [{ path: "/tmp" }, { path: "/mnt/data" }];
    orderMounts(mounts);
    expect(mounts.map((mount) => mount.path)).toEqual(["/tmp", "/mnt/data"]);
  });
});
