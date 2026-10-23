import { describe, expect, test } from "bun:test";

import {
  collectDriveStats,
  decodeDeviceNumber,
  parseDiskstats,
  parseTemperature,
  parseUptime,
} from "../src/disk-stats.js";

describe("parseDiskstats", () => {
  test("reads the counters the dashboard shows", () => {
    const line =
      "   8       0 sda 123456 789 9876543 45678 234567 890 12345678 56789 2 123456 789012";
    const counters = parseDiskstats(line).get("sda");
    expect(counters).toEqual({
      readsCompleted: 123456,
      sectorsRead: 9876543,
      writesCompleted: 234567,
      sectorsWritten: 12345678,
      ioInProgress: 2,
      busyMs: 123456,
    });
  });

  test("keeps every device and skips malformed lines", () => {
    const text = [
      "   8       0 sda 1 0 2 0 3 0 4 0 0 0 0",
      " 259       0 nvme0n1 5 0 6 0 7 0 8 0 0 0 0",
      "",
      "not a diskstats line",
    ].join("\n");
    const parsed = parseDiskstats(text);
    expect(parsed.size).toBe(2);
    expect(parsed.get("nvme0n1")?.readsCompleted).toBe(5);
  });
});

describe("parseUptime", () => {
  test("reads the first field", () => {
    expect(parseUptime("12345.67 45678.90\n")).toBe(12345.67);
  });

  test("rejects a missing or zero uptime", () => {
    expect(parseUptime("")).toBeUndefined();
    expect(parseUptime("0.00 0.00")).toBeUndefined();
  });
});

describe("parseTemperature", () => {
  test("converts millidegrees to celsius", () => {
    expect(parseTemperature("52000")).toBe(52);
    expect(parseTemperature("52500")).toBe(52.5);
    expect(parseTemperature("-5000")).toBe(-5);
  });

  test("rejects unparsable and implausible values", () => {
    expect(parseTemperature(undefined)).toBeUndefined();
    expect(parseTemperature("")).toBeUndefined();
    expect(parseTemperature("warm")).toBeUndefined();
    expect(parseTemperature("999999")).toBeUndefined();
  });
});

describe("decodeDeviceNumber", () => {
  test("splits small and large device numbers", () => {
    expect(decodeDeviceNumber(2049n)).toEqual({ major: 8, minor: 1 });
    expect(decodeDeviceNumber(66304n)).toEqual({ major: 259, minor: 0 });
    expect(decodeDeviceNumber(1114924n)).toEqual({ major: 259, minor: 300 });
  });
});

describe("collectDriveStats", () => {
  test("returns nothing when the path has no block device", () => {
    expect(collectDriveStats("/definitely/not/a/real/path")).toBeUndefined();
  });
});
