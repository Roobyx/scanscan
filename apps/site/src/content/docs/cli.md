---
title: CLI reference
description: The scanscan command surface, JSON-first and read-only.
---

Every command accepts `--json` and writes one JSON object to stdout. Global flags: `--data-dir`,
`--snapshot`.

| Command | Purpose |
|---|---|
| `scanscan scan <path...>` | Scan and index paths (`--incremental`, `--one-file-system`, `--follow-symlinks`, `--exclude <glob>`, `--apparent`, `--threads N`) |
| `scanscan daemon` | Run the Unix-socket JSON-RPC daemon (`--socket`, `--data-dir`) |
| `scanscan ls [path]` | List children (`--sort`, `--limit`) |
| `scanscan du [path]` | Subtree sizes (`--depth`, `--apparent`) |
| `scanscan top [path]` | Largest entries (`-n`, `--kind`, `--metric alloc\|apparent\|items`) |
| `scanscan find [path]` | Filter by `--size`, `--mtime`, `--ext`, `--owner`, `--dupe` |
| `scanscan tree [path]` | Print the hierarchy (`--depth`) |
| `scanscan ext [path]` | Extension breakdown |
| `scanscan diff <a> <b>` | Growth/shrink between snapshots |
| `scanscan docker` | Containers, sizes, mounts (`--stats`) |
| `scanscan export <snap>` | Export (`--format json`) |
| `scanscan snapshots` | `list`, `delete <id>`, `gc --keep N` |
| `scanscan config` | `get`, `set <key> <value>` |
| `scanscan completions <shell>` | Shell completions |

## Metrics

- **allocated** (`sizeAlloc`) — blocks actually consumed (`st_blocks * 512`); the default.
- **apparent** (`sizeApparent`) — logical file size.

Hardlinked files count once for allocated totals; sparse files report both metrics.

## Examples

```bash
scanscan scan / --json
scanscan top / -n 25 --metric alloc --json
scanscan find / --size +1G --mtime -180d --json
scanscan diff <older> <newer> --json
scanscan snapshots gc --keep 3 --json
```
