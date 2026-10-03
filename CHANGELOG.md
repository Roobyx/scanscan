# Changelog

All notable changes to scanscan are documented here. Newest entries first.

## [1.9.3] - 2026-10-03

### Fixed
- The Docker tab returned HTTP 502 while scans were running. The core daemon processed requests
  sequentially per connection, so a slow Docker call (stats across many containers) blocked the
  shared connection until the server timed out. Each request now runs on its own thread, and
  responses are written under a lock (matched by id, so ordering does not matter).

## [1.9.2] - 2026-10-03

### Added
- The scan form now exposes **exclusions** (gitignore-style globs) and an **"include other mounted
  filesystems"** toggle. By default a scan stops at filesystem boundaries, so scanning `/host`
  scans only the root filesystem and skips separately-mounted drives (`/mnt/piDrive`, etc.).

## [1.9.1] - 2026-10-03

### Added
- A per-scan **Delete** button in the header, so individual snapshots can be removed from the UI
  (previously only GC was available).

### Changed
- Reverted the experimental Parquet export: its dependency failed to resolve at build time, so it
  will return with a working approach.

## [1.9.0] - 2026-10-03

### Added
- Content-addressed block store (ADR-2). The writer chunks the fixed-size columns (nodes, subtree,
  inode) into 64K-node zstd-compressed blocks stored under `snapshots/blocks/<ab>/<hash>`, addressed
  by BLAKE3 and deduplicated. The manifest records block references; the reader decompresses blocks
  on demand with a small cache. Pre-CAS snapshots fall back to memory-mapped columns.
- `snapshots gc` now also sweeps CAS blocks no longer referenced by any live manifest.

## [1.8.0] - 2026-10-03

### Added
- Incremental rescans (`--incremental` / `options.incremental`). Snapshots now carry an inode
  column (`ino`+`dev` per node, format v2), and a rescan reuses any directory whose `(mtime, inode)`
  is unchanged by copying its subtree from the previous snapshot instead of re-reading the
  filesystem. Format v1 snapshots remain readable (the inode column is optional).

## [1.7.0] - 2026-10-02

### Added
- OpenAPI 3 document served at `GET /api/v1/openapi.json`, describing every endpoint (core, scans,
  tree, query, docker, schedules) for the web client and the docs site.

## [1.6.0] - 2026-10-02

### Added
- Scheduled scans: a server-side cron scheduler (`GET/POST/DELETE /api/v1/schedules`) that
  triggers scans on a 5-field cron expression, plus a Schedules view to add, list and delete
  schedules.

## [1.5.1] - 2026-10-02

### Fixed
- `docker.mountsFor` could hang: resolving each mount via `IndexReader::children` was O(subtree)
  per call, making N mounts O(N·n). It now builds a parent→children index once and computes each
  distinct node's subtree bytes once.

## [1.5.0] - 2026-10-02

### Added
- Docker image and volume sizes: `docker.images` / `GET /api/v1/docker/images` and `docker.volumes`
  / `GET /api/v1/docker/volumes`, shown in the Docker panel as images and volumes tables.
- Mount → index correlation: `docker.mountsFor` / `GET /api/v1/docker/mounts?scan=<id>` resolves
  each container mount's host path to a snapshot node (with subtree bytes). The Docker panel shows
  the node and clicking it drills the dashboard to that path.

## [1.4.0] - 2026-10-02

### Added
- Age × size heatmap view (PLAN §10.3 #8): `query.heatmap` / `GET /api/v1/scans/:id/heatmap`
  bins every file by age bucket (from `<1d` to `>3y`) and size bucket (from `0-1 KB` to `>1 GB`);
  the web client renders it as a heatmap with per-cell file count and bytes.

## [1.3.0] - 2026-10-02

### Added
- Global scan progress: a floating panel (bottom-right) shows running scans with live file/dir/
  byte counts and the current path, visible from every view. Starting a scan no longer blocks the
  form; the dashboard navigates to the snapshot when the scan finishes.

### Fixed
- Host drive discovery no longer reads `/proc/mounts`, which reflects the container's namespace
  even when bind-mounted. It now walks the read-only `/host` mount and detects mount points by
  device changes, so the picker lists the host's real filesystems.

## [1.2.1] - 2026-10-02

### Fixed
- The host drive picker listed the container's mounts instead of the host's. The stack now mounts
  the host's `/proc/mounts` at `/host.mounts` (`SCANSCAN_HOST_MOUNTS`) and prefers it when reading
  mounts.

## [1.2.0] - 2026-10-02

### Added
- Host drive picker in the scan form: `host.mounts` (GET `/api/v1/host/mounts`) reads the host
  mount table under `/host/proc/mounts`, filters to real block-backed filesystems, and maps each
  host mountpoint to its container path. The **Start a scan** panel lists the drives and fills the
  root when one is chosen, and explains that the host is mounted read-only at `/host`.
- `SCANSCAN_HOST_ROOT` (default `/host`) to configure the host mount location.

## [1.1.0] - 2026-10-02

### Added
- Host filesystem scanning. The compose stack bind-mounts the host read-only at `/host`
  (`HOST_MOUNT`) and defaults `SCANSCAN_ROOTS` to `/host`; the scan form pre-fills the configured
  roots. The app container runs as root so the core can read the whole host (read-only).

### Changed
- Pseudo-filesystem exclusions (`proc`, `sys`, `dev`, `run`, `snap`) are now applied under every
  root, not only `/`.

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
