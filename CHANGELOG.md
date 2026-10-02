# Changelog

All notable changes to scanscan are documented here. Newest entries first.

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
