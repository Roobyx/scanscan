---
title: Web UI guide
description: Every view, colour mode, drill-down, search and export.
---

The web client renders from a single snapshot and keeps every view coupled to the same selection.

## Views

- **Treemap** — squarified rectangles; area = size, nesting = hierarchy. Click to drill in,
  breadcrumbs to zoom out, hover for a tooltip.
- **Sunburst / Icicle / Bubble** — radial, stacked and packed reads on the same hierarchy.
- **Bars** — the top 25 entries, toggle allocated vs apparent.
- **Histogram** — file-count and size mass by size bucket.
- **Extensions / Age / Owners** — aggregated breakdowns with proportional bars.
- **Docker** — container cards, a mounts table, and live CPU/memory stats.
- **Duplicates** — metadata-identical files ranked by wasted space.
- **Diff** — pick a baseline snapshot and see grown/shrunk/added/removed paths.

## Colour modes

Colour the treemap by **extension** or by **size**. Mount/container overlays are labelled so bind
mounts and volumes are unmistakable.

## Search and export

The search panel filters the current subtree by name, extension, kind and size range. **Export**
downloads the subtree as CSV or JSON.

## Shareable state

The active view, scope, colour mode and search query are encoded in the URL hash
(`#view=icicle&scope=12&color=ext&q=log`), so any view can be linked.
