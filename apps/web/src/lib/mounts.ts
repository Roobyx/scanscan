import type { DriveInfo, ScanSummary } from "@scanscan/api-types";

/** Host paths that are infrastructure rather than user data on Linux. */
export const SYSTEM_MOUNT_PATHS = ["/tmp", "/var/log", "/boot/efi"] as const;

/** Drop trailing slashes so `/tmp/` and `/tmp` are the same mount. */
export function normalizeMountPath(path: string): string {
  if (path.length <= 1) return path;
  return path.replace(/\/+$/, "");
}

export function isSystemMount(path: string): boolean {
  return (SYSTEM_MOUNT_PATHS as readonly string[]).includes(normalizeMountPath(path));
}

/** Display name for a host mount. `/` is the absolute root of the host. */
export function mountLabel(path: string): string {
  const normalized = normalizeMountPath(path);
  return normalized === "/" ? "Absolute root" : normalized;
}

/**
 * User mounts first, system mounts last, original order otherwise.
 *
 * `Array.prototype.sort` is stable, so equal ranks keep the order the core
 * reported and the list does not jump between refreshes.
 */
export function orderMounts<T extends { path: string }>(mounts: readonly T[]): T[] {
  return [...mounts].sort((a, b) => Number(isSystemMount(a.path)) - Number(isSystemMount(b.path)));
}

/** Label for a snapshot root, mapping container paths back to host mount names. */
export function scanRootLabel(scan: ScanSummary, drives: readonly DriveInfo[]): string {
  const root = scan.roots[0] ?? "—";
  const drive = drives.find((entry) => entry.containerPath === root);
  if (drive) return mountLabel(drive.path);
  return root;
}
