---
title: Configuration
description: Environment variables and scan options.
---

## Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `SCANSCAN_DATA_DIR` | `./data` | Catalog, snapshots and journal |
| `SCANSCAN_BIND` | `127.0.0.1:8080` | Server bind address |
| `SCANSCAN_CORE_SOCKET` | `/tmp/scanscan.sock` | Unix socket for the core daemon |
| `SCANSCAN_CORE_BIN` | — | Path to the `scanscan` binary the server supervises |
| `SCANSCAN_ROOTS` | — | Default roots (comma-separated) |
| `SCANSCAN_DOCKER` | `false` | Enable Docker integration |
| `SCANSCAN_WEB_DIR` | — | Directory of built SPA assets to serve |
| `SCANSCAN_DOCKER_SOCKET` | `$DOCKER_HOST` or `/var/run/docker.sock` | Docker endpoint (`unix://`, `tcp://`) |
| `DOCKER_HOST` | — | Docker endpoint used when `SCANSCAN_DOCKER_SOCKET` is unset |

## Scan options

`incremental`, `oneFileSystem` (default true), `followSymlinks`, `exclusions` (gitignore globs),
`ignoreFiles`, `threads`, `allocated` (default true).

## Snapshots and GC

Each scan produces an immutable snapshot. `snapshots gc --keep N` (or `POST /api/v1/gc`) deletes
completed snapshots beyond the newest N. Data backup = copy `blocks/` (when present),
`snapshots/` and `catalog.db`.
