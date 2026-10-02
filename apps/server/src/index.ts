import { existsSync } from "node:fs";
import { spawn } from "node:child_process";

import { serveStatic } from "hono/bun";

import { createApp } from "./app.js";
import { readConfig } from "./config.js";
import { CoreClient } from "./core/client.js";

const config = readConfig();

// Supervise the Rust core daemon when it is bundled alongside the server.
if (config.coreBin && !existsSync(config.coreSocket)) {
  const child = spawn(
    config.coreBin,
    ["daemon", "--socket", config.coreSocket, "--data-dir", config.dataDir],
    { stdio: "inherit" },
  );
  child.on("exit", (code) => {
    console.error(JSON.stringify({ level: "error", msg: "core exited", code }));
  });
  process.on("exit", () => child.kill());
}

const core = new CoreClient(config.coreSocket);
const app = createApp({ config, core });

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
