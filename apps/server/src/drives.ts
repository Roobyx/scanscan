import { statfsSync, statSync } from "node:fs";

import type { DriveInfo, DriveStats, HostMount } from "@scanscan/api-types";

import { collectDriveStats } from "./disk-stats.js";
import { filesystemOf, readMounts } from "./mount-info.js";

/** Filesystem stats arrive as BigInt; a plain, finite number is what the wire needs. */
function toNum(value: unknown): number {
  const parsed = Number(value as number);
  return Number.isFinite(parsed) ? parsed : 0;
}

function deviceOf(path: string): string {
  try {
    return String(statSync(path).dev);
  } catch {
    return path;
  }
}

interface Capacity {
  total: number;
  free: number;
  inodesTotal?: number;
  inodesFree?: number;
}

/**
 * Capacity of the filesystem holding `path`.
 *
 * `bavail` is what an unprivileged writer can use and `bfree` what root can, so
 * a drive that is full for a normal user must read as full here: `bavail` is
 * used as-is, including a zero value.
 */
function capacityOf(path: string): Capacity | null {
  try {
    const stats = statfsSync(path);
    const bsize = toNum(stats.bsize);
    const total = bsize * toNum(stats.blocks);
    if (total <= 0) return null;
    const avial = toNum(stats.bavail);
    const free = bsize * avial;

    // Some filesystems (btrfs, for one) report no inode limit; omit those
    // rather than rendering "0 of 0".
    const inodesTotal = toNum(stats.files);
    const inodesFree = toNum(stats.ffree);

    return {
      total,
      free: Math.min(total, free),
      inodesTotal: inodesTotal > 0 ? inodesTotal : undefined,
      inodesFree: inodesTotal > 0 ? Math.min(inodesTotal, inodesFree) : undefined,
    };
  } catch {
    return null;
  }
}

function statsOf(containerPath: string, capacity: Capacity | null): DriveStats | undefined {
  const hardware = collectDriveStats(containerPath);
  const inodesTotal = capacity?.inodesTotal;
  if (hardware === undefined && inodesTotal === undefined) return undefined;

  const inodesFree = capacity?.inodesFree;
  return {
    ...(hardware ?? {}),
    inodesTotal,
    inodesFree,
    inodesUsed:
      inodesTotal !== undefined ? Math.max(0, inodesTotal - (inodesFree ?? 0)) : undefined,
  };
}

/**
 * One drive per filesystem.
 *
 * The host mounts are discovered by device-id change, so a filesystem reachable
 * at several paths yields several mounts; the shortest host path names the drive
 * because that is what a file explorer shows. A shorter path never wins at the
 * price of losing the capacity a longer path of the same filesystem reported.
 */
export function drivesFromMounts(mounts: HostMount[]): DriveInfo[] {
  const byDevice = new Map<string, DriveInfo>();
  const filesystems = readMounts();
  for (const mount of mounts) {
    const identity = deviceOf(mount.containerPath);
    const capacity = capacityOf(mount.containerPath);
    const existing = byDevice.get(identity);
    if (existing) {
      if (existing.path.length <= mount.path.length) continue;
      if (existing.totalBytes !== undefined && !capacity) continue;
    }
    const total = capacity?.total ?? 0;
    const free = capacity?.free ?? 0;

    // The core discovers mounts by device-id change and leaves the source
    // fields empty; the container's own mount table knows the filesystem type.
    const filesystem = filesystemOf(mount.containerPath, filesystems);

    byDevice.set(identity, {
      path: mount.path,
      containerPath: mount.containerPath,
      device: mount.device || filesystem?.source || "",
      fstype: mount.fstype || filesystem?.fstype || "",
      totalBytes: capacity ? total : undefined,
      freeBytes: capacity ? free : undefined,
      usedBytes: capacity ? Math.max(0, total - free) : undefined,
      stats: statsOf(mount.containerPath, capacity),
    });
  }
  const drives: DriveInfo[] = [];
  for (const drive of byDevice.values()) drives.push(drive);
  return drives.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}
