/**
 * Wire types shared by the server and web client.
 *
 * Phase 0 hand-maintains this subset. From Phase 1 these are generated from
 * `crates/scanscan-ipc` (the single source of truth) by a small generator
 * script, so the Rust core and the TypeScript server cannot drift apart.
 */

export const PROTOCOL_VERSION = 1;

export interface RpcRequest {
  jsonrpc: "2.0";
  id: number;
  method: string;
  params?: unknown;
}

export interface RpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface RpcResponse {
  jsonrpc: "2.0";
  id: number;
  result?: unknown;
  error?: RpcError;
}

export type ScanState = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface ScanOptions {
  roots: string[];
  incremental?: boolean;
  oneFileSystem?: boolean;
  followSymlinks?: boolean;
  exclusions?: string[];
  ignoreFiles?: string[];
  threads?: number;
  allocated?: boolean;
}

export interface ScanSummary {
  id: string;
  parentId?: string;
  state: ScanState;
  roots: string[];
  startedAtMs: number;
  finishedAtMs?: number;
  files: number;
  dirs: number;
  bytesApparent: number;
  bytesAlloc: number;
  errors: number;
}

export interface ScanProgress {
  scanId: string;
  state: ScanState;
  files: number;
  dirs: number;
  bytesApparent: number;
  bytesAlloc: number;
  errors: number;
  elapsedMs: number;
  etaMs?: number;
  currentPath?: string;
}

export type NodeKind = "file" | "directory" | "symlink" | "special";

export interface NodeRecord {
  id: number;
  parent?: number;
  name: string;
  kind: NodeKind;
  sizeApparent: number;
  sizeAlloc: number;
  mtimeMs: number;
  subtreeSize: number;
  children: number;
  hasChildren?: boolean;
  dockerMount?: boolean;
  error?: boolean;
}

export interface Tile {
  node: number;
  name: string;
  size: number;
  x: number;
  y: number;
  w: number;
  h: number;
  colorKey?: string;
}

export interface TilesResponse {
  snapshot: string;
  scope: number;
  depth: number;
  tiles: Tile[];
  truncated: boolean;
}

export interface HealthResponse {
  status: "ok";
  core: { connected: boolean; socket: string; version?: string };
  protocol: number;
  uptimeS: number;
}

/** Terminal scan states never transition again. */
export function isTerminalState(state: ScanState): boolean {
  return state === "completed" || state === "failed" || state === "cancelled";
}

/** JSON-RPC error codes mirrored from `scanscan-ipc::rpc::codes`. */
export const RPC_CODES = {
  parseError: -32700,
  invalidRequest: -32600,
  methodNotFound: -32601,
  invalidParams: -32602,
  internalError: -32603,
  notFound: -32001,
  conflict: -32002,
  unsupportedFormat: -32003,
} as const;
