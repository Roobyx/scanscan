const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Format a byte count using binary (1024-based) units. */
export function formatBytes(bytes: number, decimals = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const suffix = UNITS[unit] ?? "B";
  return unit === 0 ? `${value.toFixed(0)} ${suffix}` : `${value.toFixed(decimals)} ${suffix}`;
}

const COUNT_UNITS = [
  { limit: 1e12, suffix: "T" },
  { limit: 1e9, suffix: "B" },
  { limit: 1e6, suffix: "M" },
  { limit: 1e3, suffix: "k" },
] as const;

/** Compact counter, e.g. `4.1M`; exact below a thousand. */
export function formatCount(count: number): string {
  if (!Number.isFinite(count) || count <= 0) return "0";
  for (const { limit, suffix } of COUNT_UNITS) {
    if (count >= limit) return `${(count / limit).toFixed(1)}${suffix}`;
  }
  return count.toFixed(0);
}

/** Percentage of `part` within `whole`, clamped to [0, 100]. */
export function percent(part: number, whole: number): number {
  if (whole <= 0) return 0;
  return Math.min(100, Math.max(0, (part / whole) * 100));
}

/** Human age of a timestamp, e.g. `12m ago`; `never` for a missing one. */
export function formatAgo(ms: number, now = Date.now()): string {
  if (!Number.isFinite(ms) || ms <= 0) return "never";
  const deltaS = Math.max(0, Math.floor((now - ms) / 1000));
  if (deltaS < 60) return "just now";
  const minutes = Math.floor(deltaS / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}
