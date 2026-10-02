import { readFileSync } from "node:fs";

export interface ServerConfig {
  host: string;
  port: number;
  bind: string;
  dataDir: string;
  roots: string[];
  dockerEnabled: boolean;
  webDir?: string;
  coreSocket: string;
  coreBin?: string;
  version: string;
}

function readPackageVersion(): string {
  try {
    const url = new URL("../package.json", import.meta.url);
    const pkg = JSON.parse(readFileSync(url, "utf8")) as { version?: string };
    return pkg.version ?? "0.0.0";
  } catch {
    return "0.0.0";
  }
}

function parseBind(bind: string): { host: string; port: number } {
  const idx = bind.lastIndexOf(":");
  if (idx === -1) return { host: bind, port: 8080 };
  const host = bind.slice(0, idx) || "0.0.0.0";
  const port = Number.parseInt(bind.slice(idx + 1), 10);
  return { host, port: Number.isFinite(port) ? port : 8080 };
}

export type EnvLike = Record<string, string | undefined>;

export function readConfig(env: EnvLike = process.env): ServerConfig {
  const bind = env.SCANSCAN_BIND ?? "127.0.0.1:8080";
  const { host, port } = parseBind(bind);
  const roots = (env.SCANSCAN_ROOTS ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);

  return {
    host,
    port,
    bind,
    dataDir: env.SCANSCAN_DATA_DIR ?? "./data",
    roots,
    dockerEnabled: ["1", "true", "yes", "on"].includes((env.SCANSCAN_DOCKER ?? "").toLowerCase()),
    webDir: env.SCANSCAN_WEB_DIR,
    coreSocket: env.SCANSCAN_CORE_SOCKET ?? "/tmp/scanscan.sock",
    coreBin: env.SCANSCAN_CORE_BIN,
    version: readPackageVersion(),
  };
}
