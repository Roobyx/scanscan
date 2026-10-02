import starlight from "@astrojs/starlight";
import { defineConfig } from "astro/config";

export default defineConfig({
  output: "static",
  integrations: [
    starlight({
      title: "scanscan",
      description:
        "Fast, headless, read-only disk-space analyzer and indexer — the WinDirStat experience for servers.",
      customCss: ["./src/styles/custom.css"],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/Roobyx/scanscan",
        },
      ],
      sidebar: [
        {
          label: "Getting started",
          items: ["index", "getting-started", "installation"],
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
  ],
  build: { format: "directory" },
  server: { host: true, port: 4321 },
});
