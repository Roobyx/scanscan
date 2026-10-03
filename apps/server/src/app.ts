import { PROTOCOL_VERSION } from "@scanscan/api-types";
import { Hono } from "hono";
import { streamSSE } from "hono/streaming";

import type { ServerConfig } from "./config.js";
import type { CoreApi } from "./core/client.js";
import { openApiDocument } from "./openapi.js";
import type { Scheduler } from "./scheduler.js";

export interface AppDeps {
  config: ServerConfig;
  core: CoreApi;
  scheduler?: Scheduler;
  startedAt?: number;
}

const TERMINAL = new Set(["completed", "failed", "cancelled"]);

function num(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

/** Build the Hono application (transport-agnostic; easy to unit test). */
export function createApp({ config, core, scheduler, startedAt = Date.now() }: AppDeps): Hono {
  const app = new Hono();

  const coreError = (message: string, status = 502) =>
    new Response(JSON.stringify({ error: message }), {
      status,
      headers: { "content-type": "application/json" },
    });

  app.get("/api/v1/health", (c) =>
    c.json({
      status: "ok" as const,
      core: core.status(),
      protocol: PROTOCOL_VERSION,
      uptimeS: Math.floor((Date.now() - startedAt) / 1000),
    }),
  );

  app.get("/api/v1/version", (c) =>
    c.json({
      server: config.version,
      protocol: PROTOCOL_VERSION,
      runtime: `bun ${Bun.version}`,
    }),
  );

  app.get("/api/v1/config", (c) =>
    c.json({
      dataDir: config.dataDir,
      bind: config.bind,
      roots: config.roots,
      dockerEnabled: config.dockerEnabled,
    }),
  );

  app.get("/api/v1/openapi.json", (c) => c.json(openApiDocument));

  app.get("/api/v1/host/mounts", async (c) => {
    try {
      return c.json(await core.call("host.mounts"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  // ---- schedules ----
  app.get("/api/v1/schedules", (c) => c.json({ schedules: scheduler?.list() ?? [] }));

  app.post("/api/v1/schedules", async (c) => {
    if (!scheduler) return coreError("scheduler unavailable", 503);
    const body = (await c.req.json().catch(() => ({}))) as {
      roots?: string[];
      cron?: string;
      enabled?: boolean;
    };
    if (!body.roots || body.roots.length === 0 || !body.cron) {
      return c.json({ error: "roots and cron are required" }, 400);
    }
    return c.json(
      scheduler.add({ roots: body.roots, cron: body.cron, enabled: body.enabled }),
      201,
    );
  });

  app.delete("/api/v1/schedules/:id", (c) => {
    if (!scheduler) return coreError("scheduler unavailable", 503);
    return c.json({ deleted: scheduler.remove(c.req.param("id")) });
  });

  // ---- scans ----
  app.get("/api/v1/scans", async (c) => {
    try {
      return c.json(await core.call("scans.list"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.post("/api/v1/scans", async (c) => {
    try {
      const body = await c.req.json().catch(() => ({}));
      return c.json(await core.call("scans.create", body), 201);
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id", async (c) => {
    try {
      return c.json(await core.call("scans.get", { id: c.req.param("id") }));
    } catch (error) {
      return coreError(String(error), 404);
    }
  });

  app.delete("/api/v1/scans/:id", async (c) => {
    try {
      return c.json(await core.call("scans.delete", { id: c.req.param("id") }));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.post("/api/v1/scans/:id/cancel", async (c) => {
    try {
      return c.json(await core.call("scans.cancel", { id: c.req.param("id") }));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/stats", async (c) => {
    try {
      return c.json(await core.call("scans.get", { id: c.req.param("id") }));
    } catch (error) {
      return coreError(String(error), 404);
    }
  });

  app.get("/api/v1/scans/:id/progress", async (c) => {
    const id = c.req.param("id");
    const wantsStream = (c.req.header("accept") ?? "").includes("text/event-stream");
    if (!wantsStream) {
      try {
        return c.json(await core.call("scans.progress", { id }));
      } catch (error) {
        return coreError(String(error), 404);
      }
    }
    return streamSSE(c, async (stream) => {
      for (;;) {
        try {
          const progress = await core.call<{ state?: string }>("scans.progress", { id });
          await stream.writeSSE({ event: "progress", data: JSON.stringify(progress) });
          if (progress.state && TERMINAL.has(progress.state)) break;
        } catch (error) {
          await stream.writeSSE({ event: "error", data: String(error) });
          break;
        }
        await stream.sleep(500);
      }
    });
  });

  app.get("/api/v1/scans/:id/children/:nodeId", async (c) => {
    try {
      return c.json(
        await core.call("tree.children", {
          id: c.req.param("id"),
          scope: num(c.req.param("nodeId"), 0),
          sort: c.req.query("sort") ?? "size",
          limit: num(c.req.query("limit"), 500),
          offset: num(c.req.query("offset"), 0),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/tiles", async (c) => {
    try {
      return c.json(
        await core.call("tree.tiles", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          depth: num(c.req.query("depth"), 2),
          color: c.req.query("color") ?? "ext",
          metric: c.req.query("metric") ?? "alloc",
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/top", async (c) => {
    try {
      return c.json(
        await core.call("query.top", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          n: num(c.req.query("n"), 50),
          kind: c.req.query("kind"),
          metric: c.req.query("metric") ?? "alloc",
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/extensions", async (c) => {
    try {
      const result = await core.call<{
        items: Array<{ key: string; count: number; size: number }>;
      }>("query.histogram", {
        id: c.req.param("id"),
        scope: num(c.req.query("scope"), 0),
        dim: "ext",
      });
      const items = (result.items ?? []).map((bucket) => ({
        ext: bucket.key,
        count: bucket.count,
        size: bucket.size,
      }));
      return c.json({ items });
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/histogram", async (c) => {
    try {
      return c.json(
        await core.call("query.histogram", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          dim: c.req.query("dim") ?? "ext",
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/heatmap", async (c) => {
    try {
      return c.json(
        await core.call("query.heatmap", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/owners", async (c) => {
    try {
      return c.json(
        await core.call("query.histogram", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          dim: "owner",
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/age", async (c) => {
    try {
      return c.json(
        await core.call("query.histogram", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          dim: "age",
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/tree", async (c) => {
    try {
      return c.json(
        await core.call("tree.hierarchy", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          depth: num(c.req.query("depth"), 2),
          limit: num(c.req.query("limit"), 2000),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/export", async (c) => {
    try {
      const format = c.req.query("format") ?? "json";
      const scope = num(c.req.query("scope"), 0);
      const result = await core.call<{ items: Array<Record<string, unknown>> }>("query.search", {
        id: c.req.param("id"),
        scope,
        limit: num(c.req.query("limit"), 100000),
      });
      const items = result.items ?? [];
      if (format === "csv") {
        const header = "id,parent,name,kind,sizeAlloc,sizeApparent,subtreeSize,mtimeMs";
        const rows = items.map((item) =>
          [
            item["id"],
            item["parent"] ?? "",
            `"${String(item["name"] ?? "").replace(/"/g, '""')}"`,
            item["kind"],
            item["sizeAlloc"],
            item["sizeApparent"],
            item["subtreeSize"],
            item["mtimeMs"],
          ].join(","),
        );
        return c.body(`${[header, ...rows].join("\n")}\n`, 200, {
          "content-type": "text/csv; charset=utf-8",
          "content-disposition": `attachment; filename="scanscan-${c.req.param("id")}.csv"`,
        });
      }
      return c.json({ snapshot: c.req.param("id"), nodes: items });
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/search", async (c) => {
    try {
      return c.json(
        await core.call("query.search", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          name: c.req.query("q"),
          ext: c.req.query("ext"),
          kind: c.req.query("kind"),
          size_min: c.req.query("size_min") ? num(c.req.query("size_min"), 0) : undefined,
          size_max: c.req.query("size_max") ? num(c.req.query("size_max"), 0) : undefined,
          limit: num(c.req.query("limit"), 500),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/duplicates", async (c) => {
    try {
      return c.json(
        await core.call("query.duplicates", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
          mode: c.req.query("mode") ?? "name+size",
          limit: num(c.req.query("limit"), 200),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/scans/:id/diff/:otherId", async (c) => {
    try {
      return c.json(
        await core.call("query.diff", {
          a: c.req.param("otherId"),
          b: c.req.param("id"),
        }),
      );
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.post("/api/v1/gc", async (c) => {
    try {
      const body = (await c.req.json().catch(() => ({}))) as { keep?: number };
      return c.json(await core.call("snapshots.gc", { keep: body.keep ?? 3 }));
    } catch (error) {
      return coreError(String(error));
    }
  });

  // ---- docker (read-only) ----
  app.get("/api/v1/docker/status", async (c) => {
    try {
      return c.json(await core.call("docker.status"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/docker/containers", async (c) => {
    try {
      return c.json(await core.call("docker.containers"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/docker/mounts", async (c) => {
    try {
      const scan = c.req.query("scan");
      if (scan) {
        return c.json(await core.call("docker.mountsFor", { id: scan }));
      }
      return c.json(await core.call("docker.mounts"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/docker/images", async (c) => {
    try {
      return c.json(await core.call("docker.images"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/docker/volumes", async (c) => {
    try {
      return c.json(await core.call("docker.volumes"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.get("/api/v1/docker/stats", async (c) => {
    try {
      return c.json(await core.call("docker.stats"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.notFound((c) => c.json({ error: "not_found", path: c.req.path }, 404));

  return app;
}
