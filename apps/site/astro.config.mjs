import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sitemap from "@astrojs/sitemap";
import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

const siteUrl = process.env.SITE_URL ?? "http://localhost:3300";

// Starlight's search close button and Expressive Code's copy button are icon-only
// and ship without an accessible name in the static HTML. Label them after the
// build so the static audit and no-JS readers get one.
const accessibleDocsButtons = {
  name: "accessible-docs-buttons",
  hooks: {
    "astro:build:done": ({ dir }) => {
      const root = fileURLToPath(dir);
      const walk = (folder) => {
        for (const entry of fs.readdirSync(folder, { withFileTypes: true })) {
          const full = path.join(folder, entry.name);
          if (entry.isDirectory()) {
            walk(full);
            continue;
          }
          if (!entry.name.endsWith(".html")) continue;
          const html = fs.readFileSync(full, "utf8");
          const next = html
            .replace(
              /<button data-close-modal/g,
              '<button data-close-modal aria-label="Close search"',
            )
            .replace(
              /<button title="Copy to clipboard"/g,
              '<button title="Copy to clipboard" aria-label="Copy code"',
            )
            .replace(
              /(<button data-open-modal[^>]*aria-label=")Search(")/g,
              "$1Search Ctrl K$2",
            )
            .replace(
              /<kbd([^>]*)>Ctrl<\/kbd><kbd([^>]*)>K<\/kbd>/g,
              "<kbd$1>Ctrl</kbd> <kbd$2>K</kbd>",
            );
          if (next !== html) fs.writeFileSync(full, next);
        }
      };
      walk(root);
    },
  },
};

export default defineConfig({
  site: siteUrl,
  output: "static",
  integrations: [
    sitemap(),
    starlight({
      title: "scanscan",
      logo: { src: "./src/assets/logo.svg", alt: "scanscan" },
      disable404Route: true,
      description:
        "Fast, headless, read-only disk-space analyzer and indexer for servers, with a browser treemap and a JSON-first CLI.",
      customCss: ["./src/styles/fonts.css", "./src/styles/custom.css"],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/Roobyx/scanscan",
        },
      ],
      head: [
        {
          tag: "meta",
          attrs: { property: "og:image", content: new URL("/og.png", siteUrl).href },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:width", content: "1200" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:height", content: "630" },
        },
        {
          tag: "meta",
          attrs: { name: "twitter:card", content: "summary_large_image" },
        },
      ],
      sidebar: [
        {
          label: "Getting started",
          items: ["overview", "getting-started", "installation"],
        },
        {
          label: "Using scanscan",
          items: ["web-ui", "cli", "api", "docker", "agent-skill"],
        },
        {
          label: "Reference",
          items: ["architecture", "configuration", "security", "troubleshooting"],
        },
      ],
    }),
    accessibleDocsButtons,
  ],
  build: { format: "directory" },
  server: { host: true, port: 4321 },
});
