<script lang="ts">
  import type { HeatmapResponse } from "@scanscan/api-types";

  import { formatBytes } from "../lib/format.js";

  interface Props {
    heatmap: HeatmapResponse;
  }

  let { heatmap }: Props = $props();

  const maxBytes = $derived(
    Math.max(1, ...heatmap.cells.flat().map((cell) => cell.bytes)),
  );

  function intensity(bytes: number): number {
    if (bytes <= 0) return 0;
    // Log scale so small buckets remain visible next to huge ones.
    return Math.min(1, Math.log10(1 + bytes) / Math.log10(1 + maxBytes));
  }
</script>

<div class="heatmap">
  <div class="corner"></div>
  <div class="size-axis">
    {#each heatmap.sizeBuckets as label (label)}
      <span>{label}</span>
    {/each}
  </div>

  <div class="age-axis">
    {#each heatmap.ageBuckets as label (label)}
      <span>{label}</span>
    {/each}
  </div>
  <div class="grid" style="--cols: {heatmap.sizeBuckets.length}">
    {#each heatmap.cells as row, ageIndex (ageIndex)}
      {#each row as cell, sizeIndex (sizeIndex)}
        <div
          class="cell"
          style="--i: {intensity(cell.bytes)}"
          title="{heatmap.ageBuckets[ageIndex]} · {heatmap.sizeBuckets[sizeIndex]} · {cell.count.toLocaleString()} files · {formatBytes(
            cell.bytes,
          )}"
        ></div>
      {/each}
    {/each}
  </div>

  <div class="legend">
    <span class="muted">less</span>
    <span class="swatch" style="--i: 0.15"></span>
    <span class="swatch" style="--i: 0.45"></span>
    <span class="swatch" style="--i: 0.75"></span>
    <span class="swatch" style="--i: 1"></span>
    <span class="muted">more (bytes)</span>
  </div>
</div>

<style>
  .heatmap {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto 1fr auto;
    gap: 0.4rem;
    width: 100%;
    height: 100%;
    padding: 0.5rem;
    min-height: 0;
  }

  .corner {
    grid-column: 1;
    grid-row: 1;
  }

  .size-axis {
    grid-column: 2;
    grid-row: 1;
    display: grid;
    grid-template-columns: repeat(var(--cols, 8), 1fr);
    gap: 2px;
    font-size: 0.62rem;
    color: var(--muted);
    text-align: center;
  }

  .size-axis span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .age-axis {
    grid-column: 1;
    grid-row: 2;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.62rem;
    color: var(--muted);
    justify-content: space-around;
  }

  .age-axis span {
    line-height: 1;
  }

  .grid {
    grid-column: 2;
    grid-row: 2;
    display: grid;
    grid-template-columns: repeat(var(--cols, 8), 1fr);
    gap: 2px;
    min-height: 0;
  }

  .cell {
    border-radius: 3px;
    background: rgba(79, 140, 255, calc(0.06 + 0.94 * var(--i)));
    min-height: 12px;
  }

  .legend {
    grid-column: 1 / -1;
    grid-row: 3;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.65rem;
  }

  .swatch {
    width: 18px;
    height: 10px;
    border-radius: 2px;
    background: rgba(79, 140, 255, calc(0.06 + 0.94 * var(--i)));
  }
</style>
