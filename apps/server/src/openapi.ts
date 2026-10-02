/**
 * OpenAPI 3 description of the scanscan HTTP API (base path `/api/v1`).
 *
 * Hand-maintained for now and served at `/api/v1/openapi.json`; it is the
 * contract the web client and the docs site build against.
 */

const ok = { "200": { description: "OK" } };
const idParam = { name: "id", in: "path", required: true, schema: { type: "string" } };
const scopeParam = {
  name: "scope",
  in: "query",
  required: false,
  schema: { type: "integer" },
  description: "Node id to scope the query to (default: root).",
};

function scanOp(summary: string, extraParams: unknown[] = []) {
  return {
    get: {
      summary,
      parameters: [idParam, scopeParam, ...extraParams],
      responses: ok,
    },
  };
}

export const openApiDocument = {
  openapi: "3.0.3",
  info: {
    title: "scanscan API",
    version: "1.7.0",
    description:
      "Read-only disk-space analyzer and indexer. All endpoints are read-only except scan creation, cancellation, snapshot GC and schedules.",
    license: { name: "GPL-3.0-or-later" },
  },
  servers: [{ url: "/api/v1" }],
  tags: [
    { name: "core" },
    { name: "scans" },
    { name: "tree" },
    { name: "query" },
    { name: "docker" },
    { name: "schedules" },
  ],
  paths: {
    "/health": { get: { tags: ["core"], summary: "Health", responses: ok } },
    "/version": { get: { tags: ["core"], summary: "Version", responses: ok } },
    "/config": { get: { tags: ["core"], summary: "Server config", responses: ok } },
    "/host/mounts": {
      get: { tags: ["core"], summary: "Host filesystems (drive picker)", responses: ok },
    },
    "/scans": {
      get: { tags: ["scans"], summary: "List scans", responses: ok },
      post: {
        tags: ["scans"],
        summary: "Create a scan",
        requestBody: {
          required: true,
          content: {
            "application/json": {
              schema: {
                type: "object",
                required: ["roots"],
                properties: {
                  roots: { type: "array", items: { type: "string" } },
                  options: {
                    type: "object",
                    properties: {
                      incremental: { type: "boolean" },
                      oneFileSystem: { type: "boolean" },
                      followSymlinks: { type: "boolean" },
                      exclusions: { type: "array", items: { type: "string" } },
                      allocated: { type: "boolean" },
                    },
                  },
                },
              },
            },
          },
        },
        responses: { "201": { description: "Created" } },
      },
    },
    "/scans/{id}": {
      get: { tags: ["scans"], summary: "Get a scan", parameters: [idParam], responses: ok },
      delete: { tags: ["scans"], summary: "Delete a snapshot", parameters: [idParam], responses: ok },
    },
    "/scans/{id}/cancel": {
      post: { tags: ["scans"], summary: "Cancel a scan", parameters: [idParam], responses: ok },
    },
    "/scans/{id}/progress": {
      get: {
        tags: ["scans"],
        summary: "Scan progress (JSON, or SSE with Accept: text/event-stream)",
        parameters: [idParam],
        responses: ok,
      },
    },
    "/scans/{id}/children/{nodeId}": {
      get: {
        tags: ["tree"],
        summary: "List direct children",
        parameters: [idParam, { name: "nodeId", in: "path", required: true, schema: { type: "integer" } }],
        responses: ok,
      },
    },
    "/scans/{id}/tiles": {
      ...scanOp("Depth-limited treemap tiles"),
      get: {
        summary: "Depth-limited treemap tiles",
        parameters: [
          idParam,
          scopeParam,
          { name: "depth", in: "query", schema: { type: "integer" } },
          { name: "color", in: "query", schema: { type: "string", enum: ["ext", "size"] } },
        ],
        responses: ok,
      },
    },
    "/scans/{id}/tree": scanOp("Bounded nested hierarchy (sunburst/icicle/bubble)"),
    "/scans/{id}/top": scanOp("Largest entries", [
      { name: "n", in: "query", schema: { type: "integer" } },
      { name: "kind", in: "query", schema: { type: "string", enum: ["file", "directory"] } },
      { name: "metric", in: "query", schema: { type: "string", enum: ["alloc", "apparent", "items"] } },
    ]),
    "/scans/{id}/extensions": scanOp("Extension breakdown"),
    "/scans/{id}/histogram": scanOp("Histogram by dimension", [
      { name: "dim", in: "query", schema: { type: "string", enum: ["ext", "age", "owner", "size"] } },
    ]),
    "/scans/{id}/heatmap": scanOp("Age x size heatmap"),
    "/scans/{id}/owners": scanOp("Owner breakdown"),
    "/scans/{id}/age": scanOp("Age breakdown"),
    "/scans/{id}/search": scanOp("Filtered search", [
      { name: "q", in: "query", schema: { type: "string" } },
      { name: "ext", in: "query", schema: { type: "string" } },
      { name: "kind", in: "query", schema: { type: "string" } },
      { name: "size_min", in: "query", schema: { type: "integer" } },
      { name: "size_max", in: "query", schema: { type: "integer" } },
    ]),
    "/scans/{id}/duplicates": scanOp("Metadata duplicates", [
      { name: "mode", in: "query", schema: { type: "string", enum: ["name+size", "name+size+mtime"] } },
    ]),
    "/scans/{id}/diff/{otherId}": {
      get: {
        tags: ["query"],
        summary: "Diff two snapshots by path",
        parameters: [idParam, { name: "otherId", in: "path", required: true, schema: { type: "string" } }],
        responses: ok,
      },
    },
    "/scans/{id}/export": scanOp("Export a subtree", [
      { name: "format", in: "query", schema: { type: "string", enum: ["csv", "json"] } },
    ]),
    "/gc": {
      post: {
        tags: ["scans"],
        summary: "Delete completed snapshots beyond the newest N",
        requestBody: {
          content: { "application/json": { schema: { type: "object", properties: { keep: { type: "integer" } } } } },
        },
        responses: ok,
      },
    },
    "/schedules": {
      get: { tags: ["schedules"], summary: "List schedules", responses: ok },
      post: {
        tags: ["schedules"],
        summary: "Create a schedule",
        requestBody: {
          required: true,
          content: {
            "application/json": {
              schema: {
                type: "object",
                required: ["roots", "cron"],
                properties: {
                  roots: { type: "array", items: { type: "string" } },
                  cron: { type: "string", example: "0 3 * * *" },
                  enabled: { type: "boolean" },
                },
              },
            },
          },
        },
        responses: { "201": { description: "Created" } },
      },
    },
    "/schedules/{id}": {
      delete: {
        tags: ["schedules"],
        summary: "Delete a schedule",
        parameters: [idParam],
        responses: ok,
      },
    },
    "/docker/status": { get: { tags: ["docker"], summary: "Docker availability", responses: ok } },
    "/docker/containers": { get: { tags: ["docker"], summary: "Containers + sizes + mounts", responses: ok } },
    "/docker/mounts": {
      get: {
        tags: ["docker"],
        summary: "Mounts (with node correlation when ?scan=<id> is given)",
        parameters: [{ name: "scan", in: "query", schema: { type: "string" } }],
        responses: ok,
      },
    },
    "/docker/images": { get: { tags: ["docker"], summary: "Images + sizes", responses: ok } },
    "/docker/volumes": { get: { tags: ["docker"], summary: "Volumes + usage", responses: ok } },
    "/docker/stats": { get: { tags: ["docker"], summary: "Live stats sample", responses: ok } },
  },
} as const;
