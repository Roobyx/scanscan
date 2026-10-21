const EXTENSION_PALETTE = [
  "#4f8cff",
  "#3ddc97",
  "#ffb454",
  "#b98cff",
  "#ff6b6b",
  "#5ad1e6",
  "#f78fb3",
  "#9ccc65",
  "#ffd166",
  "#8d99ae",
] as const;

const UNKNOWN_COLOR = "#5b6577";

/** FNV-1a hash of a string, returned as an unsigned 32-bit integer. */
export function hashString(key: string): number {
  let hash = 2166136261;
  for (let i = 0; i < key.length; i += 1) {
    hash ^= key.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

/** Deterministic HSL color derived from an arbitrary key. */
export function colorForKey(key: string): string {
  const hue = hashString(key) % 360;
  return `hsl(${hue} 62% 52%)`;
}

/** Lowercased extension of a file name, or "" when it has none. */
export function extensionOf(name: string): string {
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) return "";
  return name.slice(dot + 1).toLowerCase();
}

/** Stable palette color for an extension key. */
export function colorForExtension(ext: string): string {
  if (ext.length === 0) return UNKNOWN_COLOR;
  const index = hashString(ext) % EXTENSION_PALETTE.length;
  return EXTENSION_PALETTE[index] ?? UNKNOWN_COLOR;
}

export type ColorMode = "ext" | "size";

/** Linear RGB interpolation for drive usage percentage with temperature stops. */
export function usageColor(usedPct: number): string {
  const t = Math.max(0, Math.min(1, usedPct / 100));

  // Stop points: (normalized t, r, g, b)
  const stops: ReadonlyArray<readonly [number, number, number, number]> = [
    [0.0, 0, 128, 0], // 0% : pure green
    [0.6, 0, 128, 0], // 60%: green (flat segment)
    [0.7, 255, 182, 52], // 70%: yellow
    [0.8, 255, 182, 52], // 80%: yellow (flat segment)
    [0.85, 191, 0, 0], // 85%: red
    [1.0, 128, 0, 0], // 100%: deep red
  ];

  let previous: readonly [number, number, number, number] | undefined;
  for (const current of stops) {
    if (previous !== undefined) {
      const [t0, r0, g0, b0] = previous;
      const [t1, r1, g1, b1] = current;
      if (t >= t0 && t <= t1) {
        const weight = t0 === t1 ? 0 : (t - t0) / (t1 - t0);
        const r = Math.round(r0 + (r1 - r0) * weight);
        const g = Math.round(g0 + (g1 - g0) * weight);
        const b = Math.round(b0 + (b1 - b0) * weight);
        return `rgb(${r}, ${g}, ${b})`;
      }
    }
    previous = current;
  }

  return "#000000";
}
