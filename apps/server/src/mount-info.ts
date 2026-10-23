import { readFileSync, statSync } from "node:fs";

const PROC_MOUNTS = "/proc/self/mounts";

export interface MountEntry {
  path: string;
  source: string;
  fstype: string;
}

/** `/proc/mounts` escapes spaces and tabs as octal sequences. */
function unescape(value: string): string {
  return value.replace(/\\([0-7]{3})/g, (_, octal: string) =>
    String.fromCharCode(Number.parseInt(octal, 8)),
  );
}

/**
 * Parse `/proc/self/mounts`.
 *
 * The file describes the container's namespace, so it cannot enumerate the
 * host's mounts, but it does name the filesystem behind a known container path
 * (including bind mounts such as `/host`), which is all this needs.
 */
export function parseMounts(text: string): MountEntry[] {
  const out: MountEntry[] = [];
  for (const line of text.split("\n")) {
    const fields = line.trim().split(/\s+/);
    if (fields.length < 3) continue;
    const source = fields[0];
    const path = fields[1];
    const fstype = fields[2];
    if (!source || !path || !fstype) continue;
    out.push({ path: unescape(path), source: unescape(source), fstype: unescape(fstype) });
  }
  return out;
}

/** Longest mountpoint that contains `path`, so nested mounts win. */
export function mountEntryFor(
  path: string,
  entries: readonly MountEntry[],
): MountEntry | undefined {
  let best: MountEntry | undefined;
  for (const entry of entries) {
    const prefix = entry.path === "/" ? "/" : `${entry.path}/`;
    if (path !== entry.path && !path.startsWith(prefix)) continue;
    if (best === undefined || entry.path.length > best.path.length) best = entry;
  }
  return best;
}

/** Read the container's mount table once per request. */
export function readMounts(): MountEntry[] {
  try {
    return parseMounts(readFileSync(PROC_MOUNTS, "utf8"));
  } catch {
    return [];
  }
}

export function filesystemOf(
  path: string,
  entries: readonly MountEntry[],
): MountEntry | undefined {
  try {
    statSync(path);
  } catch {
    return undefined;
  }
  return mountEntryFor(path, entries);
}
