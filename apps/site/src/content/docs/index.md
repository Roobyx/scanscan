---
title: scanscan
description: Fast, headless, read-only disk-space analyzer and indexer — the WinDirStat experience for servers.
template: splash
hero:
  tagline: Where did my disk space go? Scan and index entire filesystems, then explore them from any browser — without Diskover's RAM cost.
  actions:
    - text: Get started
      link: /getting-started/
      icon: right-arrow
    - text: GitHub
      link: https://github.com/Roobyx/scanscan
      icon: external
      variant: minimal
---

## Built for operators

- **Scan once, query forever.** Immutable snapshots with memory-mapped columns; folder sizes are
  O(1) and top-N is a slice scan.
- **Browser treemap at 60fps.** A WebGL/canvas treemap with depth-limited tiles — never 50M
  rectangles over the wire — plus sunburst, icicle, bubble, bars and breakdowns.
- **Understands Docker.** Container, image and volume sizes plus bind mounts and volumes mapped
  onto the index, via a read-only GET-only socket proxy.
- **CLI and agent first.** Every command emits JSON; a first-class Skill drives scans, queries and
  Docker audits.
- **Bounded memory.** The scanner streams to disk with O(depth) state.
- **Read-only by design.** No delete, no move, no telemetry. Mount the host read-only.
