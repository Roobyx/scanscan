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
| `scanscan tui [path]` | Browse children as sized bars (`--limit`, `--metric`, `--sort`) |
| `scanscan ext [path]` | Extension breakdown |
| `scanscan diff <a> <b>` | Growth/shrink between snapshots |
| `scanscan docker` | Containers, sizes, mounts (`--stats`) |
| `scanscan export <snap>` | Export (`--format csv\|json`) |
| `scanscan snapshots` | `list`, `delete <id>`, `gc --keep N` |
| `scanscan config` | `get`, `set <key> <value>` |
| `scanscan completions <shell>` | Shell completions |

## Metrics

- **allocated** (`sizeAlloc`) — blocks actually consumed (`st_blocks * 512`); the default.
- **apparent** (`sizeApparent`) — logical file size.

Hardlinked files count once for allocated totals; sparse files report both metrics.

## Terminal view

`scanscan tui [path]` is the interective terminal view: it renders the children of a path as
sized bars and reads one line command at a time, so it works over SSH, inside a pipe and on
terminals without a termios layer. An empty line pages down; a bare number opens that child.

```text
<enter> | j   page down        k        page up
u | ..        go to the parent g        go to the snapshot root
o | <n>       open child n     s        cycle the sort order
m             cycle the metric + | -    wider / narrower bars
q             quit
```

`--json` emits one page as JSON and exits, so the same view is scriptable.

## Examples

```bash
scanscan scan / --json
scanscan top / -n 25 --metric alloc --json
scanscan find / --size +1G --mtime -180d --json
scanscan diff <older> <newer> --json
scanscan snapshots gc --keep 3 --json
```
