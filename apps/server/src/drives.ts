import { statSync, statfsSync } from "node:fs";

import type { DriveInfo, HostMount } from "@scanscan/api-types";

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

/**
 * Capacity of the filesystem holding `path`.
 *
 * `bavail` is what an unprivileged writer can use and `bfree` what root can, so
 * a drive that is full for a normal user must read as full here: `bavail` is
 * used as-is, including a zero value.
 */
function capacityOf(path: string): { total: number; free: number } | null {
  try {
    const stats = statfsSync(path);
    const bsize = toNum(stats.bsize);
    const total = bsize * toNum(stats.blocks);
    if (total <= 0) return null;
    const avial = toNum(stats.bavail);
    const free = bsize * avial;
    return { total, free: Math.min(total, free) };
  } catch {
    return null;
  }
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
    byDevice.set(identity, {
      path: mount.path,
      containerPath: mount.containerPath,
      device: mount.device ?? "",
      fstype: mount.fstype ?? "",
      totalBytes: capacity ? total : undefined,
      freeBytes: capacity ? free : undefined,
      usedBytes: capacity ? Math.max(0, total - free) : undefined,
    });
  }
  const drives: DriveInfo[] = [];
  for (const drive of byDevice.values()) drives.push(drive);
  return drives.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}
