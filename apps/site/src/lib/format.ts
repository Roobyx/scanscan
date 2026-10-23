const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

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

export function formatCount(count: number): string {
  if (!Number.isFinite(count) || count <= 0) return "0";
  for (const { limit, suffix } of COUNT_UNITS) {
    if (count >= limit) return `${(count / limit).toFixed(1)}${suffix}`;
  }
  return count.toFixed(0);
}
