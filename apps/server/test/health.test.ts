import { describe, expect, test } from "bun:test";

import { createApp } from "../src/app.js";
import type { ServerConfig } from "../src/config.js";
import type { CoreApi, CoreStatus } from "../src/core/client.js";

function config(): ServerConfig {
  return {
    host: "127.0.0.1",
    port: 8080,
    bind: "127.0.0.1:8080",
    dataDir: "/data",
    roots: ["/host"],
    dockerEnabled: false,
    webDir: undefined,
    coreSocket: "/run/scanscan/scanscan.sock",
    coreBin: undefined,
    version: "0.1.0",
  };
}

/** Core stub that records calls and returns canned results. */
class StubCore implements CoreApi {
  calls: Array<{ method: string; params?: unknown }> = [];

  constructor(private readonly handler: (method: string, params?: unknown) => unknown) {}

  status(): CoreStatus {
    return { connected: true, socket: "/stub.sock" };
  }

  async call<T>(method: string, params?: unknown): Promise<T> {
    this.calls.push({ method, params });
    return this.handler(method, params) as T;
  }
}

describe("server API", () => {
  test("health reports ok and core state", async () => {
    const core = new StubCore(() => ({}));
    const app = createApp({ config: config(), core });
    const res = await app.request("/api/v1/health");
    expect(res.status).toBe(200);
    const body = (await res.json()) as { status: string; protocol: number; core: CoreStatus };
    expect(body.status).toBe("ok");
    expect(body.protocol).toBe(1);
    expect(body.core.connected).toBe(true);
  });

  test("lists scans from the core", async () => {
    const core = new StubCore(() => ({ scans: [{ id: "s1", state: "completed" }] }));
    const app = createApp({ config: config(), core });
    const res = await app.request("/api/v1/scans");
    expect(res.status).toBe(200);
    const body = (await res.json()) as { scans: Array<{ id: string }> };
    expect(body.scans[0]?.id).toBe("s1");
    expect(core.calls[0]?.method).toBe("scans.list");
  });

  test("maps tiles query params to the core", async () => {
    const core = new StubCore(() => ({ tiles: [] }));
    const app = createApp({ config: config(), core });
    const res = await app.request("/api/v1/scans/abc/tiles?scope=7&depth=3&color=ext");
    expect(res.status).toBe(200);
    const call = core.calls[0];
    expect(call?.method).toBe("tree.tiles");
    expect(call?.params).toMatchObject({ id: "abc", scope: 7, depth: 3, color: "ext" });
  });

  test("unknown route is a JSON 404", async () => {
    const core = new StubCore(() => ({}));
    const app = createApp({ config: config(), core });
    const res = await app.request("/nope");
    expect(res.status).toBe(404);
    const body = (await res.json()) as { error: string };
    expect(body.error).toBe("not_found");
  });
});
