# scanscan

> Fast, headless, web-accessible disk-space analyzer and indexer — the WinDirStat experience for
> servers, without Diskover's RAM/index cost.

`scanscan` scans and indexes filesystems, serves an interactive web client from a headless daemon,
works fully from the CLI, understands Docker container sizes and mounts, and ships with a
marketing/documentation site and an agent Skill.

**Status:** v1.0.0 — Phases 0–4 complete. See [`PLAN.md`](./PLAN.md) for the authoritative design
and roadmap. **License:** GPL-3.0-or-later. **Primary target:** Linux → macOS → Windows.

## Features

- Streaming pre-order scanner (parallel `stat`, hardlink dedup, sparse/mount flags, exclusions,
  per-entry errors) into an immutable, memory-mapped snapshot.
- Web client with treemap, sunburst, icicle, bubble, ranked bars, size histogram and
  extension/age/owner breakdowns, search, duplicates, diff/trend, and a Docker dashboard.
- JSON-first CLI and a versioned UDS JSON-RPC daemon; REST/SSE API with camelCase wire types.
- Read-only Docker integration (sizes, mounts, live stats) via a GET-only socket proxy.
- Astro + Starlight documentation site on its own port.

## Architecture

| Component | Path | Language | Responsibility |
|---|---|---|---|
| Core library | `crates/scanscan-core/` | Rust | Scanner, index (CAS blocks + manifests), query engine, Docker collector |
| CLI / daemon | `crates/scanscan-cli/` | Rust | `scanscan` binary, `daemon` subcommand, UDS JSON-RPC server |
| IPC contract | `crates/scanscan-ipc/` | Rust | Protocol types; single source of truth for the TS client |
| API server | `apps/server/` | TypeScript (Bun + Hono) | REST/SSE, scan orchestration, scheduler, static serving |
| Web client | `apps/web/` | TypeScript (Svelte 5 + Vite) | WebGL treemap and all visualizations |
| Docs/landing site | `apps/site/` | Astro | Static landing page + documentation, own container/port |
| Shared types | `packages/api-types/` | TypeScript | Types + client shared with the server |
| Agent skill | `skills/scanscan/` | Markdown + scripts | Skill definition and helper scripts |

## Quickstart (Docker)

```bash
docker compose up -d --build
# web client:  http://localhost:8080
# docs/landing: http://localhost:3300   (canonical 3000; overridable via SCANSCAN_SITE_PORT)
```

## Local development

```bash
# Rust
cargo build --workspace
cargo test --workspace

# TypeScript
bun install
bun run --filter '*' build
bun run --filter '*' test
```

## Commands

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

bun install
bun run --filter '*' check
bun run --filter '*' test
bun run --filter '*' build
```

## Documentation

- Design & roadmap: [`PLAN.md`](./PLAN.md)
- Contributor/agent conventions: [`AGENTS.md`](./AGENTS.md)
- Architecture decisions: [`docs/adr/`](./docs/adr/)

## License

GPL-3.0-or-later. See [`LICENSE`](./LICENSE).
