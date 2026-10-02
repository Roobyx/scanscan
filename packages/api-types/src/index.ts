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

export type MountKind = "bind" | "volume" | "overlayupper" | "tmpfs";

export interface MountInfo {
  containerId: string;
  containerName: string;
  kind: MountKind;
  source: string;
  destination: string;
  readWrite: boolean;
  node?: number;
  size?: number;
}

export interface ContainerInfo {
  id: string;
  name: string;
  image: string;
  state: string;
  sizeRw?: number;
  sizeRootFs?: number;
  mounts: MountInfo[];
}

export interface DockerStats {
  id: string;
  name: string;
  cpuPercent: number;
  memUsed: number;
  memLimit: number;
  netRx: number;
  netTx: number;
  blkRead: number;
  blkWrite: number;
}

export interface ImageInfo {
  id: string;
  repoTags: string[];
  size: number;
  sharedSize: number;
  containers: number;
}

export interface VolumeInfo {
  name: string;
  driver: string;
  mountpoint: string;
  size?: number;
  refCount?: number;
}

export interface HierarchyNode {
  id: number;
  name: string;
  kind: NodeKind;
  size: number;
  children?: HierarchyNode[];
}

export interface HistogramBucket {
  key: string;
  label: string;
  count: number;
  size: number;
}

export interface HeatmapCell {
  count: number;
  bytes: number;
}

export interface HeatmapResponse {
  ageBuckets: string[];
  sizeBuckets: string[];
  cells: HeatmapCell[][];
}

export interface HostMount {
  path: string;
  containerPath: string;
  device: string;
  fstype: string;
}

export interface HostMountsResponse {
  hostRoot: string;
  mounts: HostMount[];
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
