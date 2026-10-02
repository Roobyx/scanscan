<script lang="ts">
  import type { HierarchyNode } from "@scanscan/api-types";

  import { colorForKey } from "../lib/color.js";
  import { formatBytes } from "../lib/format.js";
  import { icicleRects, type IcicleRect } from "../lib/layout.js";

  interface Props {
    root: HierarchyNode;
    onselect: (node: HierarchyNode) => void;
  }

  let { root, onselect }: Props = $props();

  const rects = $derived(icicleRects(root, 3));
  const rows = $derived.by(() => {
    let maxDepth = 0;
    for (const rect of rects) maxDepth = Math.max(maxDepth, rect.depth);
    const out: IcicleRect[][] = [];
    for (let depth = 0; depth <= maxDepth; depth += 1) {
      out.push(rects.filter((rect) => rect.depth === depth));
    }
    return out;
  });
</script>

<div class="icicle" role="group" aria-label="Icicle chart of disk usage">
  {#each rows as row, depth (depth)}
    <div class="row" style="height: {100 / Math.max(rows.length, 1)}%">
      {#each row as rect (rect.node.id)}
        <button
          type="button"
          class="cell"
          style="left: {rect.x * 100}%; width: {rect.w * 100}%; background: {colorForKey(rect.node.name)}"
          title={`${rect.node.name} · ${formatBytes(rect.node.size)}`}
          onclick={() => onselect(rect.node)}
        >
          {#if rect.w > 0.05}<span>{rect.node.name}</span>{/if}
        </button>
      {/each}
    </div>
  {/each}
</div>

<style>
  .icicle {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .row {
    position: relative;
    min-height: 18px;
    border-bottom: 1px solid var(--bg);
  }

  .cell {
    position: absolute;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0 4px;
    overflow: hidden;
    border: none;
    border-right: 1px solid var(--bg);
    color: rgba(11, 14, 20, 0.92);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }

  .cell span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cell:hover {
    filter: brightness(1.08);
  }

  .cell:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
</style>
