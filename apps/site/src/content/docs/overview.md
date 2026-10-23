---
title: Overview
description: What scanscan indexes, how a snapshot is stored, and where to start.
---

scanscan answers one question: where did the disk space go? It scans a filesystem into an
immutable snapshot, serves an interactive web client from a headless daemon, and exposes the same
index through a JSON-first CLI.

## Start here

- [Getting started](/getting-started/): run the compose stack, scan a path, open the map.
- [Installation](/installation/): compose, Portainer, and from source.
- [Web UI guide](/web-ui/): every view, colour mode and drill-down.

## What a snapshot is

A snapshot is a directory of flat files, not a database. Queries read it; they never re-crawl the
filesystem.

It contains four flat files:

- `manifest.json`: metadata, stats, the extension table and scan errors.
- `nodes.bin`: fixed-size node records, with the pre-order index as the id.
- `names.bin`: concatenated names.
- `subtree.bin`: one u32 subtree size per node.

Nodes are emitted in pre-order, so every subtree is a contiguous id range. `subtree_size` is
computed on the way back up during the scan. That one column makes a folder total a single read.

## The rest of the docs

- [Architecture](/architecture/): the process boundary, the index format, and the data flow.
- [CLI](/cli/): every command, the JSON contract, and the terminal view.
- [API](/api/): REST and SSE endpoints for the web client.
- [Docker](/docker/): what is collected and how to keep the socket read-only.
- [Configuration](/configuration/): environment variables and defaults.
- [Security](/security/): read-only guarantees, permissions, and network exposure.
- [Troubleshooting](/troubleshooting/): the failure modes and what to check first.
