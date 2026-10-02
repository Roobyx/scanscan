# AGENTS.md — scanscan

Guidance for AI coding agents working in this repository. Read this before making changes.
`PLAN.md` is the authoritative design document; this file covers how to build, test, and what
must not be broken.

## What this project is

`scanscan` is a fast, headless, web-accessible disk-space analyzer and indexer — the WinDirStat
experience for servers, without Diskover's RAM/index cost. It scans and indexes filesystems,
serves an interactive web client, works fully from the CLI, understands Docker container sizes
and mounts, and ships with a marketing/documentation site and an agent Skill.

Primary target: Linux. Then macOS, then Windows. License: GPL-3.0-or-later.

## Current status

Planning is complete (`PLAN.md`). The repository is otherwise unbuilt: Phase 0 (scaffolding) has
not started. The commands below describe the target toolchain — if a path or script does not exist
yet, scaffold it as part of Phase 0 rather than assuming it is available.

## Architecture

| Component | Path | Language | Responsibility |
|---|---|---|---|
| Core library | `crates/scanscan-core/` | Rust | Scanner, index (CAS blocks + manifests), query engine, Docker collector |
| CLI / daemon | `crates/scanscan-cli/` | Rust | `scanscan` binary, `daemon` subcommand, UDS JSON-RPC server |
| IPC contract | `crates/scanscan-ipc/` | Rust | Protocol types; single source of truth for the TS client |
| API server | `apps/server/` | TypeScript (Bun + Hono) | REST/SSE, scan orchestration, scheduler, static serving, core supervision |
| Web client | `apps/web/` | TypeScript (Svelte 5 + Vite) | WebGL treemap and all visualizations |
| Docs/landing site | `apps/site/` | Astro + Starlight | Static landing page + documentation, separate container/port |
| Shared types | `packages/api-types/` | TypeScript (generated) | Types + client generated from the IPC/OpenAPI contract |
| Agent skill | `skills/scanscan/` | Markdown + scripts | Skill definition and helper scripts |
| Deployment | `deploy/docker/`, `deploy/compose/` | Docker | Dockerfiles, compose, Portainer stack |

Full design, data model, and roadmap live in `PLAN.md`. Do not re-derive architecture ad hoc.

## Essential commands

Rust (workspace root):

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo bench                       # criterion, synthetic trees
```

TypeScript (use the repo `vp` toolchain mandated by the global agent config):

```
vp install                        # only way to change dependencies/lockfile
vp test                           # unit/integration tests
vp check                          # lint + format + typecheck (Biome-backed)
vp run -r build                   # build all packages/apps
```

Local development:

```
cargo run -p scanscan-cli -- daemon --socket /tmp/scanscan.sock --data-dir ./data
vp run --filter server dev        # API server
vp run --filter web dev           # web client
```

Docker:

```
docker compose -f deploy/compose/docker-compose.yml up --build
docker compose -f deploy/compose/portainer-stack.yml config   # validate stack
```

Before reporting completion, run the full suite: `cargo test --workspace`,
`cargo clippy ... -D warnings`, `cargo fmt --check`, `vp test`, and `vp check`. Report any command
that could not be run and why.

## Code conventions

Follow the global agent config, plus these project specifics.

Rust:
- `edition = "2021"` (workspace-level). Keep the workspace dependency set lean; do not add a crate
  without a stated reason (see `PLAN.md` for the intended set).
- Errors: `thiserror` for library error enums, `anyhow` only at the CLI boundary. No `unwrap`/
  `expect` in library code on fallible paths.
- Logging via `tracing`; no `println!` for diagnostics in the core.
- The scanner must **stream** nodes to disk and keep only O(depth) state plus the hardlink set.
  Never build the full tree in memory.
- Prefer explicit integer widths matching the index format (`u32`/`u64`/`i64`); document any
  endianness-sensitive code.
- `unsafe` only where required (e.g. `statx`, mmap) with a `// SAFETY:` comment.

TypeScript:
- Biome for lint/format; `vp check` is the gate. Do not restate formatter rules in code.
- Relative imports use explicit `.js` extensions. No `baseUrl` in tsconfigs.
- Use CSS Modules (`*.module.css`) co-located with components; for Svelte, component-scoped styles
  are the accepted equivalent. No large global stylesheets; no duplicated selectors.
- Comment only where reasoning is non-obvious. Do not add comments on everything.

## Generated & restricted files

Do not edit by hand:
- `packages/api-types/` — generated from `crates/scanscan-ipc` / OpenAPI.
- `**/dist/`, `apps/web/build/`, `apps/site/dist/` — build output (`vp run -r build`).
- `pnpm-lock.yaml` / `bun.lock` / `Cargo.lock` — only via `vp install` / `cargo` when dependencies
  change; commit lockfile changes together with the dependency change.
- `deploy/compose/portainer-stack.yml` is generated/kept in sync from `docker-compose.yml` where
  practical — update both together.

## Critical boundaries

These are safety and correctness invariants. Do not violate them.

1. **Read-only.** scanscan never deletes, moves, or modifies scanned files, and exposes no
   destructive CLI/API operation in v1. Do not add cleanup/mutation features without explicit
   approval and a feature flag.
2. **Docker is read-only.** Only GET calls to the Docker Engine API. Never mutate containers,
   images, or volumes. The socket is mounted read-only.
3. **No heavyweight index.** Do not introduce Elasticsearch/OpenSearch, a document DB, or a
   per-file SQL table. The index is immutable CAS column blocks + manifests + mmap. SQLite is for
   the small catalog only.
4. **Bounded memory.** The scanner streams; queries mmap and page blocks. Do not load whole
   snapshots into RAM.
5. **Index format is versioned.** Any change to node columns, blocks, or the manifest requires a
   format version bump, golden-test updates, and a documented migration path. Do not silently
   change the layout.
6. **Pre-order invariant.** Nodes are emitted pre-order so subtrees are contiguous and
   `subtree_size` is exact. Anything that reorders nodes must recompute children/sibling links and
   `subtree_size` and be covered by tests.
7. **No telemetry, no external network calls** from core, server, or web at runtime. Fully
   offline-capable.
8. **Paths are validated against indexed roots** in the API; never allow traversal outside them.

## Testing

- Add or update tests for changed public behavior, including validation and failure paths
  (permissions, missing files, no Docker socket).
- Rust: unit tests for scanner edge cases (hardlinks, sparse files, symlinks/loops, mount
  boundaries, unicode/deep paths); property tests for squarified layout (area conservation, no
  overlap); golden tests for the block/manifest format; fuzz the manifest parser; criterion
  benchmarks on synthetic trees.
- TypeScript: `vp test` for server routes and the IPC client; schema snapshot tests; Playwright for
  E2E flows (scan fixture fs → treemap → drill-down → search → diff → docker panel).
- Fixtures must be deterministic. Prefer behavior-based assertions over private implementation
  details. Validate size math against `du` on fixtures within a documented tolerance.
- Docker changes: run the compose smoke test and `portainer-stack.yml config` validation.

## Versioning & changelog

Required for every meaningful change, per the global config:
- Bump `version` in the affected `package.json` / crate `Cargo.toml` (patch = fix, minor = feature).
- Add a newest-first entry to the affected `CHANGELOG.md` (new version, today's date, 2–3 concise
  bullets). Version and top changelog entry stay in sync.
- Small design/UI changes get a patch bump; reserve minor bumps for new features or notable
  behavior changes.
- Keep migrations and generated files in the same change as the code that requires them.

## Deployment notes

- App container exposes `8080`; docs/landing site exposes `3000` (host) → `80` (container).
- Host filesystems are bind-mounted read-only; `docker.sock` is mounted read-only.
- Prefer the `docker-socket-proxy` (GET-only) variant for least privilege; document limitations for
  rootless Docker/Podman.
- The core needs broad read access (root or `CAP_DAC_READ_SEARCH`) to scan the whole filesystem.
  Document this; do not weaken it silently.

## Agent skill

`skills/scanscan/SKILL.md` is part of the product. When CLI commands, flags, JSON output, or API
endpoints change, update the skill (`SKILL.md`, `reference.md`, `examples.md`, scripts) in the same
change. The skill must stay JSON-first, read-only, and safe.

## Git workflow

- One feature per branch; merge to the main feature branch when allowed.
- Never stage secrets or `.env`. Keep the plan (`PLAN.md`) updated when architecture decisions
  change, and record non-trivial decisions as ADRs in `docs/`.
- Only commit when explicitly asked.

## Completion report

After completing a task, report:
- Summary of behavior changed (user-visible where relevant)
- Modified files
- Acceptance criteria status
- Assumptions and known limitations
- Remaining risks / follow-ups
