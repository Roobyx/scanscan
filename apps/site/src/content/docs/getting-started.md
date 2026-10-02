---
title: Getting started
description: Scan a filesystem and explore it in the browser in a few minutes.
---

scanscan answers one question fast: **where did my disk space go?** It scans and indexes
filesystems into an immutable snapshot, then serves an interactive web client and a JSON-first CLI.

## 1. Run it

```bash
docker compose up -d --build
```

- Web client + API: <http://localhost:8080>
- Landing + docs: <http://localhost:3300>

The compose file mounts a GET-only `docker-socket-proxy` so container sizes and mounts are visible
without exposing the Docker socket to the app.

## 2. Scan something

Open the web client and press **Scan**, or use the API:

```bash
curl -s -X POST http://localhost:8080/api/v1/scans \
  -H 'content-type: application/json' \
  -d '{"roots":["/etc"]}'
```

## 3. Explore

- **Treemap** — area is size; click a rectangle to drill in.
- **Sunburst / Icicle / Bubble** — other reads on the same hierarchy.
- **Bars / Histogram / Extensions / Age / Owners** — ranked and aggregated views.
- **Docker** — per-container size, mounts and live stats.
- **Duplicates / Diff** — cleanup candidates and growth between two snapshots.

## Next

- [Installation](/installation/) — compose, Portainer, binary.
- [Web UI guide](/web-ui/) — every view and colour mode.
- [CLI reference](/cli/) — the full command surface.
