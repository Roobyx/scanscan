# scanscan reference

JSON-first, read-only. Every command accepts `--json` and writes one JSON object to stdout.

## Commands

| Command | Purpose | Key flags |
|---|---|---|
| `scanscan scan <path...>` | Scan + index paths | `--incremental`, `--one-file-system`, `--follow-symlinks`, `--exclude <glob>`, `--ignore-file <path>`, `--apparent`, `--threads N` |
| `scanscan daemon` | Run the UDS JSON-RPC daemon | `--socket <path>`, `--data-dir <path>` |
| `scanscan ls [path]` | List children | `--sort size\|name\|mtime`, `--limit N` |
| `scanscan du [path]` | Subtree sizes | `--depth N`, `--apparent` |
| `scanscan top [path]` | Largest entries | `-n N`, `--kind file\|dir`, `--metric alloc\|apparent\|items` |
| `scanscan find [path]` | Filtered search | `--size`, `--mtime`, `--ext`, `--owner`, `--regex`, `--dupe` |
| `scanscan tree [path]` | Print hierarchy | `--depth N` |
| `scanscan ext [path]` | Extension breakdown | — |
| `scanscan diff <a> <b>` | Compare two snapshots | — |
| `scanscan docker` | Containers, sizes, mounts | `--stats` |
| `scanscan export <snap>` | Export a snapshot | `--format csv\|json\|parquet` |
| `scanscan snapshots` | List/delete/GC snapshots | `list`, `delete <id>`, `gc --keep N` |
| `scanscan config` | Get/set configuration | `get`, `set <key> <value>` |

## Metrics

- **allocated** (`size_alloc`) — blocks actually consumed on disk (`st_blocks * 512`). This is
  what "space used" means. Default metric.
- **apparent** (`size_app`) — logical file size (`st_size`).

Hardlinked files are counted once for the allocated total. Sparse files report both metrics.

## HTTP API (v1)

```
GET  /api/v1/health
GET  /api/v1/version
GET  /api/v1/config
GET  /api/v1/scans
POST /api/v1/scans                    # {roots, options}
GET  /api/v1/scans/:id
DELETE /api/v1/scans/:id
POST /api/v1/scans/:id/cancel
GET  /api/v1/scans/:id/progress       # SSE
GET  /api/v1/scans/:id/children/:nodeId
GET  /api/v1/scans/:id/tiles?path=&depth=&w=&h=&color=
GET  /api/v1/scans/:id/top?n=&kind=&metric=
GET  /api/v1/scans/:id/search?q=&size_min=&size_max=&mtime_before=&ext=&owner=
GET  /api/v1/scans/:id/diff/:otherId
GET  /api/v1/docker/containers
GET  /api/v1/docker/stats             # SSE
GET  /api/v1/docker/mounts
```

## Snapshots & diffs

A scan produces an immutable snapshot. `--incremental` reuses unchanged blocks from the parent
snapshot (structural sharing), making repeat scans of a mostly-static disk cheap. `diff` joins
two snapshots by path and reports grown/shrunk/new/deleted.

## Exit codes

`0` success, non-zero on error. In `--json` mode errors are printed as
`{"ok": false, "error": "..."}`.
