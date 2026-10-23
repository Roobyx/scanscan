import { describe, expect, test } from "bun:test";

import type { DrivesResponse, HostMountsResponse } from "@scanscan/api-types";

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

class StubCore implements CoreApi {
  constructor(private readonly handler: (method: string) => unknown) {}

  status(): CoreStatus {
    return { connected: true, socket: "/stub.sock" };
  }

  async call<T>(method: string): Promise<T> {
    return this.handler(method) as T;
  }
}

function mountsOf(mounts: HostMountsResponse["mounts"]): HostMountsResponse {
  return { hostRoot: "/host", mounts };
}

async function drivesFrom(core: CoreApi): Promise<DrivesResponse> {
  const app = createApp({ config: config(), core });
  const res = await app.request("/api/v1/host/drives");
  expect(res.status).toBe(200);
  return (await res.json()) as DrivesResponse;
}

describe("host drives", () => {
  const cwd = process.cwd();

  test("reports capacity for a filesystem the host exposes", async () => {
    const core = new StubCore(() =>
      mountsOf([{ path: "/", containerPath: cwd, device: "1", fstype: "ext4" }]),
    );
    const body = await drivesFrom(core);
    const drive = body.drives[0];
    expect(drive?.path).toBe("/");
    expect(drive?.containerPath).toBe(cwd);
    expect(drive?.fstype).toBe("ext4");
    expect((drive?.totalBytes ?? 0) > 0).toBe(true);
    expect((drive?.freeBytes ?? 0) <= (drive?.totalBytes ?? 0)).toBe(true);
    expect((drive?.usedBytes ?? 0) + (drive?.freeBytes ?? 0)).toBe(drive?.totalBytes);
  });

  test("keeps one drive per filesystem, shortest path first", async () => {
    const core = new StubCore(() =>
      mountsOf([
        { path: "/mnt/data", containerPath: cwd, device: "1", fstype: "ext4" },
        { path: "/", containerPath: cwd, device: "1", fstype: "ext4" },
      ]),
    );
    const body = await drivesFrom(core);
    expect(body.drives.length).toBe(1);
    expect(body.drives[0]?.path).toBe("/");
  });

  test("reports inode usage alongside capacity", async () => {
    const core = new StubCore(() =>
      mountsOf([{ path: "/", containerPath: cwd, device: "1", fstype: "ext4" }]),
    );
    const body = await drivesFrom(core);
    const stats = body.drives[0]?.stats;
    expect(stats).toBeDefined();
    if (stats?.inodesTotal !== undefined) {
      expect(stats.inodesTotal).toBeGreaterThan(0);
      expect(stats.inodesFree ?? 0).toBeLessThanOrEqual(stats.inodesTotal);
      expect((stats.inodesUsed ?? 0) + (stats.inodesFree ?? 0)).toBe(stats.inodesTotal);
    }
  });

  test("fills the filesystem type and source from the mount table", async () => {
    const core = new StubCore(() =>
      mountsOf([{ path: "/", containerPath: cwd, device: "", fstype: "" }]),
    );
    const body = await drivesFrom(core);
    const drive = body.drives[0];
    expect((drive?.fstype.length ?? 0) > 0).toBe(true);
    expect((drive?.device.length ?? 0) > 0).toBe(true);
  });

  test("sorts drives by host path", async () => {
    const core = new StubCore(() =>
      mountsOf([
        { path: "/mnt/data", containerPath: `${cwd}/nope-a`, device: "", fstype: "" },
        { path: "/backup", containerPath: `${cwd}/nope-b`, device: "", fstype: "" },
      ]),
    );
    const body = await drivesFrom(core);
    const paths = body.drives.map((drive) => drive.path).join(",");
    expect(paths).toBe("/backup,/mnt/data");
  });

  test("falls back to the configured root while the core is down", async () => {
    const core = new StubCore(() => {
      throw new Error("core connection closed");
    });
    const body = await drivesFrom(core);
    expect(body.hostRoot).toBe("/host");
    expect(body.drives.length).toBe(1);
    expect(body.drives[0]?.containerPath).toBe("/host");
    expect(body.warning).toBe("Error: core connection closed");
  });
});
