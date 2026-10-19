---
title: Web UI guide
description: Every view, colour mode, drill-down, search and export.
---

The web client renders from a single snapshot and keeps every view coupled to the same selection.

## Views

- **Drives** — the default view. One card per host filesystem mounted into the container: a
  usage bar with free and used space, the filesystem type, the newest snapshot's size and age,
  and Open/Scan actions. A card scanns only its own filesystem.
- **Treemap** — squarified rectangles; area = size, nesting = hierarchy. Click to drill in,
  breadcrumbs to zoom out, hover for a tooltip.
- **Sunburst / Icicle / Bubble** — radial, stacked and packed reads on the same hierarchy.
- **Bars** — the top 25 entries, toggle allocated vs apparent.
- **Histogram** — file-count and size mass by size bucket.
- **Extensions / Age / Owners** — aggregated breakdowns with proportional bars.
- **Docker** — container cards, a mounts table, and live CPU/memory stats.
- **Duplicates** — metadata-identical files ranked by wasted space.
- **Diff** — pick a baseline snapshot and see grown/shrunk/added/removed paths.

## Tree panel

The panel beside the chart is a tree of the whole snapshot, always rooted at the root the scan
started from — not at the folder you drilled into — so the path you walked is always visible and
the row of the current scope is highlighted. Folders expand with the chevron; rows carry a
file/folder icon (file icons are tinted by extension) instead of a kind column, and are sortable by
name, allocated, apparent or item count.

## Drive stats

The chart panel head shows the selected snapshot's filesystem: a small usage bar with free, total
and used percentage, so a treemap can be read against the drive it came from.

## Colour modes

Colour the treemap by **extension** or by **size**. Mount/container overlays are labelled so bind
mounts and volumes are unmistakable.

## Search and export

The search panel filters the current subtree by name, extension, kind and size range. **Export**
downloads the subtree as CSV or JSON.

## Shareable state

The active view, scope, colour mode and search query are encoded in the URL hash
(`#view=icicle&scope=12&color=ext&q=log`), so any view can be linked.
