# Changelog

All notable changes to scanscan are documented here. Newest entries first.

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
