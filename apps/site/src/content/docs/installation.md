---
title: Installation
description: Docker compose, Portainer, and running from source.
---

## Docker compose (recommended)

```bash
git clone https://github.com/Roobyx/scanscan
cd scanscan
docker compose up -d --build
```

| Service | Port | Purpose |
|---|---|---|
| `scanscan` | `8080` | API + web client (bundles the Rust core) |
| `scanscan-site` | `3300` | Landing + docs (canonical 3000; override with `SCANSCAN_SITE_PORT`) |
| `docker-socket-proxy` | — | GET-only Docker API access |

The host filesystems are bind-mounted read-only and the app never mounts `docker.sock` directly.

## Portainer

Use `deploy/compose/portainer-stack.yml`: **Stacks → Add stack → paste/upload → set environment
variables → Deploy**. See `deploy/compose/.env.example` for `SCANSCAN_PORT`,
`SCANSCAN_SITE_PORT`, `HOST_MOUNT` and `DOCKER_HOST`.

## From source

```bash
# Rust core + CLI
cargo build --workspace --release

# TypeScript (Bun workspaces)
bun install
bun run --filter '*' build
```

Run the core daemon and the server:

```bash
scanscan daemon --socket /tmp/scanscan.sock --data-dir ./data
SCANSCAN_CORE_SOCKET=/tmp/scanscan.sock bun run --filter '@scanscan/server' start
```

## Permissions

The core needs broad read access to scan the whole filesystem: run it as root, or grant
`CAP_DAC_READ_SEARCH`. Docker integration additionally needs access to the Docker API (use the
GET-only socket proxy).
