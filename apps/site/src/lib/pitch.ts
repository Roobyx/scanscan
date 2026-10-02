export const TAGLINE = "The WinDirStat experience for servers.";

export function tagline(): string {
  return TAGLINE;
}

export const COMPARISONS = [
  { tool: "scanscan", index: "CAS blocks + mmap", ram: "< 1 GB", web: "yes", docker: "yes" },
  { tool: "WinDirStat", index: "none", ram: "low", web: "no", docker: "no" },
  { tool: "Diskover", index: "Elasticsearch", ram: "4 GB+", web: "yes", docker: "no" },
  { tool: "gdu / dua", index: "none", ram: "low", web: "ttyd", docker: "no" },
] as const;
