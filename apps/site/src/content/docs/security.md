---
title: Security & privacy
description: Read-only by design, no telemetry, least-privilege Docker.
---

## Read-only

scanscan never deletes, moves, or modifies scanned files, and exposes no destructive operation in
v1. The only writes are to its own data directory (snapshots and the catalog).

## No telemetry

There are no external network calls at runtime. scanscan is fully offline-capable.

## Docker

Only `GET` requests are issued to the Docker Engine API. Prefer the GET-only socket proxy over
mounting `docker.sock`. The proxy is the only service that touches the socket.

## Permissions

The core needs broad read access to scan the whole filesystem: run it as root or grant
`CAP_DAC_READ_SEARCH`. Host filesystems are mounted read-only.

## Network exposure

Bind localhost by default. For LAN exposure, put scanscan behind a reverse proxy with TLS. There is
no authentication in v1.

## Path validation

API paths are validated against indexed roots; traversal outside them is rejected.
