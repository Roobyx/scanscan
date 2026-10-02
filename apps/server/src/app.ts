import { PROTOCOL_VERSION } from "@scanscan/api-types";
import { Hono } from "hono";
import { streamSSE } from "hono/streaming";

import type { ServerConfig } from "./config.js";
import type { CoreApi } from "./core/client.js";

export interface AppDeps {
  config: ServerConfig;
  core: CoreApi;
  startedAt?: number;
}

const TERMINAL = new Set(["completed", "failed", "cancelled"]);

function num(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

/** Build the Hono application (transport-agnostic; easy to unit test). */
export function createApp({ config, core, startedAt = Date.now() }: AppDeps): Hono {
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
      return c.json(
        await core.call("query.histogram", {
          id: c.req.param("id"),
          scope: num(c.req.query("scope"), 0),
        }),
      );
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
      return c.json(await core.call("docker.mounts"));
    } catch (error) {
      return coreError(String(error));
    }
  });

  app.notFound((c) => c.json({ error: "not_found", path: c.req.path }, 404));

  return app;
}
