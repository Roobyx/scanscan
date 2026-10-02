import type {
  ContainerInfo,
  DockerStats,
  HealthResponse,
  HeatmapResponse,
  HierarchyNode,
  HistogramBucket,
  HostMountsResponse,
  MountInfo,
  NodeKind,
  NodeRecord,
  ScanProgress,
  ScanSummary,
  TilesResponse,
} from "@scanscan/api-types";

const BASE = "/api/v1";

/** Human-readable message from an unknown thrown value. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  headers.set("accept", "application/json");
  const res = await fetch(`${BASE}${path}`, { ...init, headers });
  if (!res.ok) {
    let message = `HTTP ${res.status} ${res.statusText}`.trim();
    try {
      const body: unknown = await res.json();
      if (body && typeof body === "object") {
        const record = body as Record<string, unknown>;
        const detail = record["error"] ?? record["message"];
        if (typeof detail === "string" && detail.length > 0) message = detail;
      }
    } catch {
      // Non-JSON error body: keep the HTTP status message.
    }
    throw new Error(message);
  }
  return (await res.json()) as T;
}

export function getHealth(): Promise<HealthResponse> {
  return request<HealthResponse>("/health");
}

export interface ServerConfig {
  dataDir: string;
  bind: string;
  roots: string[];
  dockerEnabled: boolean;
}

export function getConfig(): Promise<ServerConfig> {
  return request<ServerConfig>("/config");
}

export function getHostMounts(): Promise<HostMountsResponse> {
  return request<HostMountsResponse>("/host/mounts");
}

export async function listScans(): Promise<ScanSummary[]> {
  const body = await request<{ scans: ScanSummary[] }>("/scans");
  return body.scans;
}

export function createScan(
  roots: string[],
  options?: { incremental?: boolean },
): Promise<ScanSummary> {
  return request<ScanSummary>("/scans", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(options ? { roots, options } : { roots }),
  });
}

export function getScan(id: string): Promise<ScanSummary> {
  return request<ScanSummary>(`/scans/${encodeURIComponent(id)}`);
}

export function getProgress(id: string): Promise<ScanProgress> {
  return request<ScanProgress>(`/scans/${encodeURIComponent(id)}/progress`);
}

export interface TilesQuery {
  scope: number | string;
  depth?: number;
  w?: number;
  h?: number;
  color?: "ext" | "size";
}

export function getTiles(id: string, query: TilesQuery): Promise<TilesResponse> {
  const params = new URLSearchParams();
  params.set("scope", String(query.scope));
  if (query.depth !== undefined) params.set("depth", String(query.depth));
  if (query.w !== undefined) params.set("w", String(Math.round(query.w)));
  if (query.h !== undefined) params.set("h", String(Math.round(query.h)));
  if (query.color !== undefined) params.set("color", query.color);
  return request<TilesResponse>(`/scans/${encodeURIComponent(id)}/tiles?${params.toString()}`);
}

export interface ChildrenQuery {
  sort?: string;
  limit?: number;
  offset?: number;
}

export interface ChildrenResponse {
  parent: NodeRecord;
  total: number;
  offset: number;
  items: NodeRecord[];
}

export function getChildren(
  id: string,
  nodeId: number,
  query?: ChildrenQuery,
): Promise<ChildrenResponse> {
  const params = new URLSearchParams();
  if (query?.sort !== undefined) params.set("sort", query.sort);
  if (query?.limit !== undefined) params.set("limit", String(query.limit));
  if (query?.offset !== undefined) params.set("offset", String(query.offset));
  const qs = params.toString();
  const suffix = qs.length > 0 ? `?${qs}` : "";
  return request<ChildrenResponse>(`/scans/${encodeURIComponent(id)}/children/${nodeId}${suffix}`);
}

export interface TopQuery {
  n?: number;
  scope?: number | string;
  kind?: NodeKind;
  metric?: "alloc" | "apparent";
}

export async function getTop(id: string, query?: TopQuery): Promise<NodeRecord[]> {
  const params = new URLSearchParams();
  if (query?.n !== undefined) params.set("n", String(query.n));
  if (query?.scope !== undefined) params.set("scope", String(query.scope));
  if (query?.kind !== undefined) params.set("kind", query.kind);
  if (query?.metric !== undefined) params.set("metric", query.metric);
  const qs = params.toString();
  const suffix = qs.length > 0 ? `?${qs}` : "";
  const body = await request<{ items: NodeRecord[] }>(
    `/scans/${encodeURIComponent(id)}/top${suffix}`,
  );
  return body.items;
}

export interface ExtensionStat {
  ext: string;
  count: number;
  size: number;
}

export async function getExtensions(id: string): Promise<ExtensionStat[]> {
  const body = await request<{ items: ExtensionStat[] }>(
    `/scans/${encodeURIComponent(id)}/extensions`,
  );
  return body.items;
}

export function getHeatmap(id: string, scope: number): Promise<HeatmapResponse> {
  return request<HeatmapResponse>(`/scans/${encodeURIComponent(id)}/heatmap?scope=${scope}`);
}

export type HistogramDim = "ext" | "age" | "owner" | "size";

export interface HistogramResponse {
  dim: string;
  items: HistogramBucket[];
}

export function getHistogram(
  id: string,
  dim: HistogramDim,
  scope: number | string = 0,
): Promise<HistogramResponse> {
  const params = new URLSearchParams();
  params.set("dim", dim);
  params.set("scope", String(scope));
  return request<HistogramResponse>(
    `/scans/${encodeURIComponent(id)}/histogram?${params.toString()}`,
  );
}

export interface TreeResponse {
  scope: number;
  depth: number;
  root: HierarchyNode;
}

export function getTree(
  id: string,
  scope: number | string,
  depth = 3,
  limit = 2000,
): Promise<TreeResponse> {
  const params = new URLSearchParams();
  params.set("scope", String(scope));
  params.set("depth", String(depth));
  params.set("limit", String(limit));
  return request<TreeResponse>(`/scans/${encodeURIComponent(id)}/tree?${params.toString()}`);
}

export interface SearchQuery {
  scope?: number | string;
  q?: string;
  ext?: string;
  kind?: NodeKind;
  size_min?: number;
  size_max?: number;
  limit?: number;
}

export async function searchNodes(id: string, params: SearchQuery): Promise<NodeRecord[]> {
  const query = new URLSearchParams();
  if (params.scope !== undefined) query.set("scope", String(params.scope));
  if (params.q !== undefined && params.q.length > 0) query.set("q", params.q);
  if (params.ext !== undefined && params.ext.length > 0) query.set("ext", params.ext);
  if (params.kind !== undefined) query.set("kind", params.kind);
  if (params.size_min !== undefined) query.set("size_min", String(params.size_min));
  if (params.size_max !== undefined) query.set("size_max", String(params.size_max));
  if (params.limit !== undefined) query.set("limit", String(params.limit));
  const body = await request<{ items: NodeRecord[] }>(
    `/scans/${encodeURIComponent(id)}/search?${query.toString()}`,
  );
  return body.items;
}

export function exportUrl(
  id: string,
  format: "csv" | "json" = "csv",
  scope: number | string = 0,
): string {
  const params = new URLSearchParams();
  params.set("format", format);
  params.set("scope", String(scope));
  return `${BASE}/scans/${encodeURIComponent(id)}/export?${params.toString()}`;
}

export interface DupGroup {
  key: string;
  count: number;
  size: number;
  wasted: number;
  items: NodeRecord[];
}

export interface DuplicatesQuery {
  scope?: number;
  mode?: string;
  limit?: number;
}

export interface DuplicatesResponse {
  mode: string;
  groups: DupGroup[];
}

export function getDuplicates(id: string, opts?: DuplicatesQuery): Promise<DuplicatesResponse> {
  const params = new URLSearchParams();
  if (opts?.scope !== undefined) params.set("scope", String(opts.scope));
  if (opts?.mode !== undefined) params.set("mode", opts.mode);
  if (opts?.limit !== undefined) params.set("limit", String(opts.limit));
  const qs = params.toString();
  const suffix = qs.length > 0 ? `?${qs}` : "";
  return request<DuplicatesResponse>(`/scans/${encodeURIComponent(id)}/duplicates${suffix}`);
}

export interface DiffEntry {
  path: string;
  before: number;
  after: number;
  delta: number;
  kind: string;
}

export interface DiffTotals {
  before: number;
  after: number;
  delta: number;
  added: number;
  removed: number;
}

export interface DiffResult {
  a: string;
  b: string;
  grown: DiffEntry[];
  shrunk: DiffEntry[];
  added: DiffEntry[];
  removed: DiffEntry[];
  totals: DiffTotals;
}

export function getDiff(id: string, otherId: string): Promise<DiffResult> {
  return request<DiffResult>(
    `/scans/${encodeURIComponent(id)}/diff/${encodeURIComponent(otherId)}`,
  );
}

export interface GcResult {
  removed: string[];
}

export function runGc(keep: number): Promise<GcResult> {
  return request<GcResult>("/gc", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ keep }),
  });
}

export async function getDockerContainers(): Promise<ContainerInfo[]> {
  const body = await request<{ containers: ContainerInfo[] }>("/docker/containers");
  return body.containers;
}

export async function getDockerMounts(): Promise<MountInfo[]> {
  const body = await request<{ mounts: MountInfo[] }>("/docker/mounts");
  return body.mounts;
}

export async function getDockerStats(): Promise<DockerStats[]> {
  const body = await request<{ containers: DockerStats[] }>("/docker/stats");
  return body.containers;
}
