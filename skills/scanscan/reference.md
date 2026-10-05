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
| `scanscan find [path]` | Filtered search | `--size`, `--mtime`, `--ext`, `--owner`, `--dupe` |
| `scanscan tree [path]` | Print hierarchy | `--depth N` |
| `scanscan ext [path]` | Extension breakdown | — |
| `scanscan diff <a> <b>` | Compare two snapshots by path | — |
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
GET  /api/v1/scans/:id/extensions
GET  /api/v1/scans/:id/histogram?dim=ext|age|owner|size&scope=
GET  /api/v1/scans/:id/owners
GET  /api/v1/scans/:id/age
GET  /api/v1/scans/:id/search?q=&ext=&kind=&size_min=&size_max=&limit=
GET  /api/v1/scans/:id/duplicates?mode=name+size|name+size+mtime&limit=
GET  /api/v1/scans/:id/diff/:otherId
GET  /api/v1/scans/:id/export?format=csv|json
POST /api/v1/gc                        # { keep } — delete snapshots beyond the newest N
GET  /api/v1/docker/containers
GET  /api/v1/docker/mounts
GET  /api/v1/docker/stats
GET  /api/v1/docker/status
GET  /api/v1/docker/locate?q=          # find the container owning a path / overlay2 id
```

## Node labels

When Docker is reachable at scan time, directories under `/var/lib/docker/overlay2/<id>`
carry a `label` naming the owning container, image and compose stack/service, e.g.
`web (nginx:latest) · myapp/web`. Layers shared by several containers read
`shared by N containers`. The label is present on `NodeRecord.label` and `Tile.label`
(omitted when absent) and is read-only metadata; it never affects sizes. Snapshots written
without Docker simply have no labels.

## Snapshots & diffs

A scan produces an immutable snapshot. `--incremental` reuses unchanged blocks from the parent
snapshot (structural sharing), making repeat scans of a mostly-static disk cheap. `diff` joins
two snapshots by path and reports grown/shrunk/new/deleted.

## Exit codes

`0` success, non-zero on error. In `--json` mode errors are printed as
`{"ok": false, "error": "..."}`.
