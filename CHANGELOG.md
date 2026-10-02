# Changelog

All notable changes to scanscan are documented here. Newest entries first.

## [1.0.1] - 2026-10-02

### Fixed
- The dashboard had no visible way to start a new scan once a completed scan was auto-selected.
  Added an always-visible **+ New scan** button in the header.

## [1.0.0] - 2026-10-02

### Added
- Phase 4: Astro + Starlight documentation site — a splash landing page plus getting started,
  installation, web UI, CLI, HTTP API, Docker, agent-skill, architecture, configuration, security
  and troubleshooting pages.
- Release workflow (manual `workflow_dispatch`) producing multi-arch (linux/amd64, linux/arm64)
  images pushed to GHCR.

### Changed
- The CI workflow is manual-only (`workflow_dispatch`) until GitHub Actions is configured for the
  repository.

## [0.4.0] - 2026-10-02

### Added
- Phase 3: metadata duplicate detection (name+size, optionally +mtime) with wasted-space ranking,
  exposed via `query.duplicates`, `GET /scans/:id/duplicates`, `scanscan find --dupe`, and a
  Duplicates view.
- Snapshot diff/trend: `query.diff` joins two snapshots by relative path and reports
  grown/shrunk/added/removed with totals; exposed via `GET /scans/:id/diff/:otherId`, `scanscan
  diff`, and a Diff view.
- Snapshot garbage collection (`snapshots.gc`, `POST /api/v1/gc`, `scanscan snapshots gc`) that
  keeps the newest N completed snapshots.

## [0.3.0] - 2026-10-02

### Added
- Phase 2 views: sunburst, icicle, bubble, ranked bars, size histogram, and extension/age/owner
  breakdowns, alongside the treemap; a Docker dashboard with container cards, mounts and live
  stats; and a search/filter panel.
- URL-addressable state (`#view=…&scope=…&color=…&q=…`), a virtualized ranked table, and CSV/JSON
  export.
- Backend: histogram dimensions (`ext|age|owner|size`), bounded hierarchy for radial/icicle/bubble
  layouts, Docker non-streaming stats, and `uid`/`gid`/`mode` capture in the scanner.
- Least-privilege Docker access: the collector accepts `unix://` and `tcp://` endpoints, and the
  reference stack ships a GET-only `docker-socket-proxy` sidecar instead of mounting `docker.sock`.

### Changed
- The IPC/HTTP wire types now serialize camelCase, matching `packages/api-types` and the web client
  (previously snake_case, which the browser could not read).

## [0.2.0] - 2026-10-02

### Added
- Phase 1 MVP: streaming pre-order Rust scanner with parallel `stat`, hardlink dedup, sparse-file
  and mount-boundary flags, exclusions (glob + pseudo-fs defaults), per-entry error recording, and
  throttled progress.
- Snapshot index format v1 (`manifest.json` + mmap'd `nodes.bin`/`names.bin`/`subtree.bin`) with
  `subtree_size` computed on the way back up, plus a query engine (children, top-N, extension
  histogram, depth-limited tiles) and a squarified treemap layout (area/overlap property tests).
- UDS JSON-RPC daemon (`scans.*`, `tree.*`, `query.*`, `docker.*`) and a full CLI
  (`scan|daemon|ls|du|top|find|tree|ext|diff|docker|export|snapshots|config`).
- Read-only Docker Engine API collector (containers, sizes, mounts) over the Unix socket.
- Server REST/SSE wiring to the core over a persistent UDS JSON-RPC client, and a Svelte 5 web
  client: canvas treemap with drill-down, ranked table, breadcrumbs, and a scan launcher.

## [0.1.0] - 2026-10-02

### Added
- Phase 0 foundation: Rust workspace (`scanscan-core`, `scanscan-cli`, `scanscan-ipc`) with the
  `Filesystem` abstraction, configuration, and the JSON-RPC 2.0 protocol types.
- TypeScript monorepo: Bun + Hono API server (`/api/v1/health|version|config`), Svelte 5 + Vite
  web client shell, and an Astro landing site.
- `packages/api-types` shared wire types with tests.
- Docker: multi-stage `Dockerfile.server` (Rust core + server + web) and `Dockerfile.site`
  (Astro + nginx), plus the root `docker-compose.yml` used by the Portainer stack.
- CI workflow (Rust fmt/clippy/test, TypeScript lint/check/test/build, Docker image builds).
- Agent skill skeleton (`skills/scanscan/`) and the first ADR.
