import { describe, expect, test } from "bun:test";

import { createApp } from "../src/app.js";
import type { ServerConfig } from "../src/config.js";
import { CoreClient } from "../src/core/client.js";

function deps() {
  const config: ServerConfig = {
    host: "127.0.0.1",
    port: 8080,
    bind: "127.0.0.1:8080",
    dataDir: "/data",
    roots: ["/host"],
    dockerEnabled: false,
    webDir: undefined,
    coreSocket: "/run/scanscan/scanscan.sock",
    version: "0.1.0",
  };
  return { config, core: new CoreClient(config.coreSocket) };
}

describe("server API", () => {
  test("health reports ok and core state", async () => {
    const app = createApp(deps());
    const res = await app.request("/api/v1/health");
    expect(res.status).toBe(200);
    const body = (await res.json()) as { status: string; protocol: number };
    expect(body.status).toBe("ok");
    expect(body.protocol).toBe(1);
  });

  test("version endpoint returns server version", async () => {
    const app = createApp(deps());
    const res = await app.request("/api/v1/version");
    const body = (await res.json()) as { server: string };
    expect(body.server).toBe("0.1.0");
  });

  test("unknown route is a JSON 404", async () => {
    const app = createApp(deps());
    const res = await app.request("/nope");
    expect(res.status).toBe(404);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("not_found");
  });
});
