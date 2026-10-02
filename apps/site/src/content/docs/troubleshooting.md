---
title: Troubleshooting
description: Common issues and how to resolve them.
---

## The Docker view is empty

The Docker socket is not reachable. Check that `docker-socket-proxy` is running and
`DOCKER_HOST` points at it (`tcp://docker-socket-proxy:2375`). Rootless Docker and Podman are
best-effort.

## Scans miss files

The core lacks read permission on some directories. Run it as root or grant
`CAP_DAC_READ_SEARCH`. Unreadable entries are recorded as errors and the scan continues; check the
snapshot's `errors` count.

## The core shows as "idle"

The server has not connected to the daemon yet. Confirm `SCANSCAN_CORE_SOCKET` matches the socket
the daemon bound, and that `SCANSCAN_CORE_BIN` points at the `scanscan` binary.

## The site port is already in use

Set `SCANSCAN_SITE_PORT` to a free host port (the canonical port is 3000).

## Sizes differ from `du`

scanscan reports **allocated** size (`st_blocks * 512`) by default and counts hardlinks once.
Compare with `du` (also allocated, hardlink-aware) rather than `du --apparent-size`. Sparse files
report both metrics.

## Reset the index

Stop the stack and remove the data volume, then rescan. scanscan never touches scanned files.
