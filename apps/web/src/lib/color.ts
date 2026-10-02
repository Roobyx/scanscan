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
