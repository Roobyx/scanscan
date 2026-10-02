---
title: Agent skill
description: Drive scanscan safely from an AI agent — JSON-first and read-only.
---

`skills/scanscan/` is a first-class Skill that lets agents investigate disk usage safely.

## When to use

- A disk is full or nearly full.
- "What is using space under `/var`?"
- Docker disk audits (container/image/volume sizes and mounts).
- Large, old (stale) files as cleanup candidates.

## Core workflow

```bash
scanscan scan /var --json
scanscan du /var --depth 3 --json
scanscan top /var -n 50 --json
scanscan find /var --size +1G --mtime -180d --json
scanscan docker --json
```

Always pass `--json` when the output feeds another step. See the skill's `reference.md` for every
command and endpoint, and `examples.md` for task recipes.

## Safety

- **Read-only.** scanscan never deletes or moves files, and exposes no destructive operation.
- Never run destructive cleanup (`rm`, `docker prune`) from results without explicit human
  confirmation. Report candidates; let a human act.
- Respect configured exclusions and indexed roots.
