---
title: Architecture
description: Rust core, TypeScript server, and the index format.
---

scanscan is split at a versioned process boundary.

| Component | Language | Responsibility |
|---|---|---|
| `scanscan-core` | Rust | Scanner, index (snapshots + columns), query engine, Docker collector |
| `scanscan-cli` | Rust | `scanscan` binary and the UDS JSON-RPC `daemon` |
| `scanscan-ipc` | Rust | Protocol types — the single source of truth |
| `apps/server` | TypeScript (Bun + Hono) | REST/SSE, orchestration, static serving |
| `apps/web` | TypeScript (Svelte 5 + Vite) | Treemap and all visualizations |
| `packages/api-types` | TypeScript | Shared wire types mirroring `scanscan-ipc` |

## Index format (v1)

A snapshot is a directory:

```
snapshots/<id>/
  manifest.json   # metadata, stats, extension table, errors
  nodes.bin       # fixed-size node records; id = index (pre-order)
  names.bin       # concatenated names
  subtree.bin     # u32 subtree size per node
```

Nodes are emitted in pre-order, so every subtree is a contiguous id range. `subtree_size` is
computed on the way back up during the scan. The format is versioned; any layout change bumps
`FORMAT_VERSION`.

## Data flow

```
scan → pre-order nodes → snapshot → query engine (mmap) → JSON-RPC → REST/SSE → browser
```

The server supervises the core daemon over a Unix socket and relays progress to browsers via SSE.
