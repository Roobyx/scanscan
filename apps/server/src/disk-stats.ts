import { readdirSync, readFileSync, realpathSync, statSync } from "node:fs";
import { basename, dirname, join } from "node:path";

import type { DriveStats } from "@scanscan/api-types";

const SYS_CLASS_BLOCK = "/sys/class/block";
const SYS_DEV_BLOCK = "/sys/dev/block";
const SYS_CLASS_HWMON = "/sys/class/hwmon";
const PROC_DISKSTATS = "/proc/diskstats";
const PROC_UPTIME = "/proc/uptime";

/** `/proc/diskstats` always counts sectors in 512-byte units. */
const SECTOR_BYTES = 512;
const MAX_SLAVE_DEPTH = 4;

export interface DiskCounters {
  readsCompleted: number;
  sectorsRead: number;
  writesCompleted: number;
  sectorsWritten: number;
  ioInProgress: number;
  busyMs: number;
}

export interface DeviceNumber {
  major: number;
  minor: number;
}

function readText(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8").trim();
  } catch {
    return undefined;
  }
}

function readDir(path: string): string[] {
  try {
    return readdirSync(path);
  } catch {
    return [];
  }
}

function real(path: string): string | undefined {
  try {
    return realpathSync(path);
  } catch {
    return undefined;
  }
}

function nonNegative(raw: string | undefined): number | undefined {
  if (raw === undefined || raw.length === 0) return undefined;
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0 ? value : undefined;
}

function field(fields: readonly string[], index: number): number {
  const raw = fields[index];
  if (raw === undefined) return 0;
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0 ? value : 0;
}

/** Parse `/proc/diskstats`, keeping only the counters the dashboard shows. */
export function parseDiskstats(text: string): Map<string, DiskCounters> {
  const out = new Map<string, DiskCounters>();
  for (const line of text.split("\n")) {
    const fields = line.trim().split(/\s+/);
    if (fields.length < 14) continue;
    const name = fields[2];
    if (!name) continue;
    out.set(name, {
      readsCompleted: field(fields, 3),
      sectorsRead: field(fields, 5),
      writesCompleted: field(fields, 7),
      sectorsWritten: field(fields, 9),
      ioInProgress: field(fields, 11),
      busyMs: field(fields, 12),
    });
  }
  return out;
}

/** First field of `/proc/uptime`, in seconds. */
export function parseUptime(text: string): number | undefined {
  const first = text.trim().split(/\s+/)[0];
  if (first === undefined) return undefined;
  const seconds = Number(first);
  return Number.isFinite(seconds) && seconds > 0 ? seconds : undefined;
}

/** hwmon reports millidegrees; reject values no disk should report. */
export function parseTemperature(raw: string | undefined): number | undefined {
  if (raw === undefined || raw.length === 0) return undefined;
  const milli = Number(raw);
  if (!Number.isFinite(milli)) return undefined;
  const celsius = Math.round(milli / 100) / 10;
  if (celsius < -40 || celsius > 150) return undefined;
  return celsius;
}

/** Decode a Linux `dev_t` the way glibc does, including large minors. */
export function decodeDeviceNumber(dev: bigint): DeviceNumber {
  const major = Number(((dev >> 8n) & 0xfffn) | ((dev >> 32n) & ~0xfffn));
  const minor = Number((dev & 0xffn) | ((dev >> 12n) & 0xfff00n));
  return { major, minor };
}

function deviceNumberOf(path: string): DeviceNumber | undefined {
  try {
    return decodeDeviceNumber(statSync(path, { bigint: true }).dev);
  } catch {
    return undefined;
  }
}

function kernelNameOf(number: DeviceNumber): string | undefined {
  const resolved = real(join(SYS_DEV_BLOCK, `${number.major}:${number.minor}`));
  return resolved ? basename(resolved) : undefined;
}

/** A partition resolves to its whole disk; `dm-0` and `md0` stay as they are. */
function wholeDiskOf(name: string): string {
  const resolved = real(join(SYS_CLASS_BLOCK, name));
  if (!resolved) return name;
  const isPartition = readText(join(resolved, "partition")) !== undefined;
  return isPartition ? basename(dirname(resolved)) : basename(resolved);
}

/** Physical disks underneath a device-mapper, RAID or partitioned device. */
function backingDisks(name: string): string[] {
  const leaves: string[] = [];
  const seen = new Set<string>();

  const visit = (current: string, depth: number): void => {
    if (depth > MAX_SLAVE_DEPTH || seen.has(current)) return;
    seen.add(current);
    const slaves = readDir(join(SYS_CLASS_BLOCK, current, "slaves"));
    if (slaves.length === 0) {
      leaves.push(current);
      return;
    }
    for (const slave of slaves) visit(slave, depth + 1);
  };

  visit(name, 0);

  const disks: string[] = [];
  for (const leaf of leaves) {
    const disk = wholeDiskOf(leaf);
    if (!disks.includes(disk)) disks.push(disk);
  }
  return disks;
}

/** Prefer a disk with a `device` node: that is where model and sensors live. */
function physicalDisk(disks: readonly string[]): string | undefined {
  for (const disk of disks) {
    if (real(join(SYS_CLASS_BLOCK, disk, "device")) !== undefined) return disk;
  }
  return disks[0];
}

function transportOf(disk: string): string | undefined {
  if (disk.startsWith("nvme")) return "nvme";
  if (disk.startsWith("xvd")) return "xen";
  if (disk.startsWith("vd")) return "virtio";
  if (disk.startsWith("mmcblk")) return "mmc";
  if (disk.startsWith("zram") || disk.startsWith("ram")) return "ram";
  if (disk.startsWith("dm-")) return "device-mapper";
  if (disk.startsWith("md")) return "raid";
  if (disk.startsWith("loop")) return "loop";

  const resolved = real(join(SYS_CLASS_BLOCK, disk));
  if (!resolved) return undefined;
  if (resolved.includes("/usb")) return "usb";
  if (resolved.includes("/ata")) return "sata";
  if (resolved.includes("/nvme")) return "nvme";
  if (resolved.includes("/virtio")) return "virtio";
  return "scsi";
}

/** `temp1_input` directly, or inside a `hwmon*` child of the device. */
function readTemperatureFrom(base: string): number | undefined {
  const direct = parseTemperature(readText(join(base, "temp1_input")));
  if (direct !== undefined) return direct;

  for (const entry of readDir(base)) {
    if (!entry.startsWith("hwmon")) continue;
    const value = parseTemperature(readText(join(base, entry, "temp1_input")));
    if (value !== undefined) return value;
  }
  return undefined;
}

/**
 * Sensors are bound to the device node, not to the block name, so a hwmon
 * class entry is matched by resolving both sides to the same device path.
 */
function temperatureOf(disk: string): number | undefined {
  const devicePath = join(SYS_CLASS_BLOCK, disk, "device");
  const direct = readTemperatureFrom(devicePath);
  if (direct !== undefined) return direct;

  const device = real(devicePath);
  if (device === undefined) return undefined;

  for (const entry of readDir(SYS_CLASS_HWMON)) {
    const base = join(SYS_CLASS_HWMON, entry);
    const owner = real(join(base, "device"));
    if (owner === undefined) continue;
    const matches =
      owner === device || owner.startsWith(`${device}/`) || device.startsWith(`${owner}/`);
    if (!matches) continue;
    const value = parseTemperature(readText(join(base, "temp1_input")));
    if (value !== undefined) return value;
  }
  return undefined;
}

/**
 * Tier-1 drive counters from sysfs and procfs.
 *
 * These reads need no privileges and no host `/dev` mount: sysfs and
 * `/proc/diskstats` are host-wide even inside a container. Returns `undefined`
 * when the path has no block device behind it (non-Linux, virtio without sysfs,
 * or a path that does not exist), so callers can omit the field entirely.
 */
export function collectDriveStats(containerPath: string): DriveStats | undefined {
  const number = deviceNumberOf(containerPath);
  if (number === undefined) return undefined;

  const kernelName = kernelNameOf(number);
  if (kernelName === undefined) return undefined;

  const disks = backingDisks(kernelName);
  const disk = physicalDisk(disks);
  if (disk === undefined) return undefined;

  const stats: DriveStats = {
    kernelName,
    model: readText(join(SYS_CLASS_BLOCK, disk, "device", "model")),
    vendor: readText(join(SYS_CLASS_BLOCK, disk, "device", "vendor")),
    serial: readText(join(SYS_CLASS_BLOCK, disk, "device", "serial")),
    transport: transportOf(disk),
    logicalBlockSize: nonNegative(
      readText(join(SYS_CLASS_BLOCK, disk, "queue", "logical_block_size")),
    ),
    physicalBlockSize: nonNegative(
      readText(join(SYS_CLASS_BLOCK, disk, "queue", "physical_block_size")),
    ),
    temperatureC: temperatureOf(disk),
  };

  if (disks.length > 1 || disks[0] !== kernelName) stats.backingDevices = disks;

  const rotational = readText(join(SYS_CLASS_BLOCK, disk, "queue", "rotational"));
  if (rotational === "0" || rotational === "1") stats.rotational = rotational === "1";

  const counters = parseDiskstats(readText(PROC_DISKSTATS) ?? "").get(kernelName);
  if (counters !== undefined) {
    stats.readsCompleted = counters.readsCompleted;
    stats.writesCompleted = counters.writesCompleted;
    stats.bytesRead = counters.sectorsRead * SECTOR_BYTES;
    stats.bytesWritten = counters.sectorsWritten * SECTOR_BYTES;
    stats.ioInProgress = counters.ioInProgress;
    stats.busyMs = counters.busyMs;

    const uptime = parseUptime(readText(PROC_UPTIME) ?? "");
    if (uptime !== undefined) {
      stats.busyPct = Math.min(100, Math.max(0, (counters.busyMs / (uptime * 1000)) * 100));
    }
  }

  return stats;
}
