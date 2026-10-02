import { PROTOCOL_VERSION } from "@scanscan/api-types";
import { Hono } from "hono";

import type { ServerConfig } from "./config.js";
import { CoreClient } from "./core/client.js";

export interface AppDeps {
  config: ServerConfig;
  core: CoreClient;
  startedAt?: number;
}

/** Build the Hono application (transport-agnostic; easy to unit test). */
export function createApp({ config, core, startedAt = Date.now() }: AppDeps): Hono {
  const app = new Hono();

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

  app.notFound((c) => c.json({ error: "not_found", path: c.req.path }, 404));

  return app;
}
