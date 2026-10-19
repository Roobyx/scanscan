import type { HistogramBucket } from "@scanscan/api-types";

/** A coarse file-type group over extensions, for the Extensions overview. */
export interface FileTypeGroupDef {
  id: string;
  label: string;
  color: string;
  extensions: string[];
}

/** Aggregated space and count of one file-type group within a scope. */
export interface FileTypeGroup {
  id: string;
  label: string;
  color: string;
  count: number;
  size: number;
}

export const OTHER_GROUP = "other";

const OTHER_COLOR = "#5b6577";

export const FILE_TYPES: FileTypeGroupDef[] = [
  {
    id: "photos",
    label: "Photos",
    color: "#4f8cff",
    extensions: [
      "jpg",
      "jpeg",
      "jfif",
      "pjp",
      "heic",
      "heif",
      "avif",
      "webp",
      "gif",
      "tif",
      "tiff",
      "bmp",
      "icu",
      "svg",
      "psd",
      "xcf",
      "raw",
      "cr2",
      "cr3",
      "nef",
      "arw",
      "dng",
      "orf",
      "raf",
      "rw2",
      "jxl",
      "jxl2",
    ],
  },
  {
    id: "videos",
    label: "Videos",
    color: "#b98cff",
    extensions: [
      "mp4",
      "m4v",
      "mkv",
      "mov",
      "avi",
      "wmv",
      "webm",
      "mpeg",
      "mpg",
      "flv",
      "3gp",
      "vob",
      "mts",
      "m2ts",
    ],
  },
  {
    id: "audio",
    label: "Audio",
    color: "#5ad1e6",
    extensions: [
      "mp3",
      "flac",
      "wav",
      "aac",
      "ogg",
      "oga",
      "opus",
      "m4a",
      "aif",
      "aiff",
      "wma",
      "mid",
      "midi",
    ],
  },
  {
    id: "packages",
    label: "Packages",
    color: "#ffb454",
    extensions: [
      "deb",
      "rpm",
      "apk",
      "apks",
      "msix",
      "msi",
      "dmq",
      "snap",
      "flatpak",
      "flatpakref",
      "zip",
      "tar",
      "gz",
      "tgz",
      "xz",
      "bz2",
      "zst",
      "7z",
      "rar",
      "lz",
      "lz4",
      "cab",
      "whl",
    ],
  },
  {
    id: "executables",
    label: "Executables",
    color: "#ff6b6b",
    extensions: ["exe", "dl", "so", "dylib", "bin", "out", "jar", "wasm", "ko", "elf", "com"],
  },
  {
    id: "documents",
    label: "Documents",
    color: "#3ddc97",
    extensions: [
      "pdf",
      "docx",
      "doc",
      "xlsx",
      "xls",
      "ppx",
      "ppt",
      "odt",
      "ods",
      "odp",
      "txt",
      "md",
      "csv",
      "rtf",
      "epub",
      "mobi",
      "azw3",
      "djv",
    ],
  },
  {
    id: "caches",
    label: "Caches / logs",
    color: "#9ccc65",
    extensions: ["cace", "tmp", "log", "bak", "old", "swp", "dum", "core", "pid", "lok", "journal"],
  },
] as const;

/** Group id an extension belongs to; unknown and empty extensions are `other`. */
export function groupOfExtension(ext: string): string {
  const key = ext.toLowerCase();
  for (const group of FILE_TYPES) {
    for (const candidate of group.extensions) {
      if (candidate === key) return group.id;
    }
  }
  return OTHER_GROUP;
}

/** Human label for a group id, including `other`. */
export function fileTypeLabel(groupId: string): string {
  if (groupId === OTHER_GROUP) return "Others";
  return FILE_TYPES.find((group) => group.id === groupId)?.label ?? "Others";
}

/**
 * Aggregated size/count per file-type group.
 *
 * Groups with no files are dropped, so the overview never shows a zero row. The
 * result is sorted by size, with `Others` always last so the named types stay
 * readable and the remainder does not push them down.
 */
export function groupByFileType(items: readonly HistogramBucket[]): FileTypeGroup[] {
  const groups: FileTypeGroup[] = FILE_TYPES.map((group) => ({
    id: group.id,
    label: group.label,
    color: group.color,
    count: 0,
    size: 0,
  }));
  const others: FileTypeGroup = {
    id: OTHER_GROUP,
    label: "Others",
    color: OTHER_COLOR,
    count: 0,
    size: 0,
  };
  for (const bucket of items) {
    const id = groupOfExtension(bucket.key);
    const target = id === OTHER_GROUP ? others : groups.find((group) => group.id === id);
    if (!target) continue;
    target.count += bucket.count;
    target.size += bucket.size;
  }
  const filled = [...groups, others].filter((group) => group.count > 0);
  return filled.sort((a, b) => {
    if (a.id === OTHER_GROUP) return 1;
    if (b.id === OTHER_GROUP) return -1;
    return b.size - a.size;
  });
}
