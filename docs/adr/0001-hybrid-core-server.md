# ADR-1: Hybrid Rust core + TypeScript server

**Status:** Accepted (Phase 0)

## Context

scanscan needs maximum scan throughput and bounded memory for tens of millions of files, but also
a fast-moving web/API layer with shared types and easy SSE streaming. A single language forces a
trade-off: Rust is ideal for the scanner/index but slower to iterate on HTTP; TypeScript is ideal
for the API/UI but cannot meet the scan performance and memory budgets.

## Decision

Split the system at a versioned process boundary:

- **Rust** owns scanning, indexing, querying, and Docker collection (`scanscan-core`), exposed
  through a `scanscan` binary with a `daemon` subcommand speaking JSON-RPC 2.0 over a Unix-domain
  socket.
- **TypeScript (Bun + Hono)** owns HTTP, orchestration, scheduling, and static serving.
- **`scanscan-ipc`** is the single source of truth for the protocol; `packages/api-types` mirrors
  it for the server and web client.

## Consequences

- Clean separation: the core can run privileged while the server does not.
- The server language is swappable (e.g. Go) without touching the core.
- Two runtimes to build and CI (mitigated by one multi-stage Dockerfile and generated types).
- A bespoke format and protocol must be versioned and kept in sync (accepted; this is the
  core differentiator, per ADR-2).
