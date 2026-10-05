import { type ChildProcess, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createConnection } from "node:net";

import { serveStatic } from "hono/bun";

import { createApp } from "./app.js";
import { readConfig } from "./config.js";
import { CoreClient } from "./core/client.js";
import { Scheduler } from "./scheduler.js";

const config = readConfig();

/** True when a live listener answers on the socket; a stale socket file is false. */
function coreSocketAlive(socketPath: string, timeoutMs = 300): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = createConnection(socketPath);
    let settled = false;
    const finish = (alive: boolean) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(alive);
    };
    socket.setTimeout(timeoutMs);
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
    socket.once("timeout", () => finish(false));
  });
}

/**
 * Supervise the bundled Rust core daemon.
 *
 * The daemon can die on its own (e.g. an OOM kill) while the server keeps
 * running and leaves a stale socket behind. Gating startup on the socket file's
 * existence meant it was never respawned and every call then failed with
 * ECONNREFUSED. Probe for a live listener instead, and restart on exit so the
 * server recovers without a container restart.
 */
function superviseCore(bin: string, socketPath: string, dataDir: string): void {
  let child: ChildProcess | null = null;
  let backoff = 1_000;
  let startedAt = 0;

  const restart = () => {
    if (Date.now() - startedAt > 30_000) backoff = 1_000;
    const delay = backoff;
    backoff = Math.min(backoff * 2, 30_000);
    setTimeout(() => void ensureCore(), delay);
  };

  const spawnCore = () => {
    startedAt = Date.now();
    let handled = false;
    const failed = (message: string, code: number | null, signal: string | null) => {
      if (handled) return;
      handled = true;
      child = null;
      console.error(JSON.stringify({ level: "error", msg: message, code, signal }));
      restart();
    };
    const proc = spawn(bin, ["daemon", "--socket", socketPath, "--data-dir", dataDir], {
      stdio: "inherit",
    });
    child = proc;
    proc.on("error", (error) => failed(`core spawn failed: ${error.message}`, null, null));
    proc.on("exit", (code, signal) => failed("core exited; restarting", code, signal));
  };

  const ensureCore = async () => {
    if (child) return;
    if (await coreSocketAlive(socketPath)) return;
    spawnCore();
  };

  void ensureCore();
  process.on("exit", () => child?.kill());
}

if (config.coreBin) {
  superviseCore(config.coreBin, config.coreSocket, config.dataDir);
}

const core = new CoreClient(config.coreSocket);
const scheduler = new Scheduler(core);
scheduler.start();
const app = createApp({ config, core, scheduler });

// Keep the core connection warm so /health reports the real core state, the
// first request is not slowed by a cold connect, and the client reconnects on
// its own after the supervisor respawns a daemon that died.
const connectCore = (): void => {
  core
    .call("core.health")
    .catch(() => undefined)
    .finally(() => {
      setTimeout(connectCore, 5000);
    });
};
connectCore();

if (config.webDir && existsSync(config.webDir)) {
  app.use("/*", serveStatic({ root: config.webDir }));
  app.get("*", serveStatic({ path: `${config.webDir}/index.html` }));
}

console.log(
  JSON.stringify({
    level: "info",
    msg: "scanscan server listening",
    bind: config.bind,
    dataDir: config.dataDir,
    webDir: config.webDir ?? null,
    coreBin: config.coreBin ?? null,
    version: config.version,
  }),
);

export default {
  port: config.port,
  hostname: config.host,
  fetch: app.fetch,
};
