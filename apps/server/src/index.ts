import { existsSync } from "node:fs";

import { serveStatic } from "hono/bun";

import { createApp } from "./app.js";
import { readConfig } from "./config.js";
import { CoreClient } from "./core/client.js";

const config = readConfig();
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
    version: config.version,
  }),
);

export default {
  port: config.port,
  hostname: config.host,
  fetch: app.fetch,
};
