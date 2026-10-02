<script lang="ts">
  import type { NodeRecord } from "@scanscan/api-types";

  import { colorForKey } from "../lib/color.js";
  import { formatBytes } from "../lib/format.js";

  interface Props {
    items: NodeRecord[];
    onselect: (node: NodeRecord) => void;
  }

  let { items, onselect }: Props = $props();

  let metric = $state<"alloc" | "apparent">("alloc");

  function metricValue(node: NodeRecord): number {
    return metric === "alloc" ? node.sizeAlloc : node.sizeApparent;
  }

  const rows = $derived.by(() => {
    const sorted = items
      .slice()
      .sort((a, b) => metricValue(b) - metricValue(a))
      .slice(0, 25);
    const max = sorted.reduce((acc, node) => Math.max(acc, metricValue(node)), 0) || 1;
    return sorted.map((node) => ({
      node,
      value: metricValue(node),
      pct: (metricValue(node) / max) * 100,
    }));
  });
</script>

<div class="bars">
  <div class="metric-toggle" role="group" aria-label="Metric">
    <button type="button" class:active={metric === "alloc"} onclick={() => (metric = "alloc")}>
      Allocated
    </button>
    <button type="button" class:active={metric === "apparent"} onclick={() => (metric = "apparent")}>
      Apparent
    </button>
  </div>
  <ol class="list">
    {#each rows as row (row.node.id)}
      <li>
        <button
          type="button"
          class="row"
          title={`${row.node.name} · ${formatBytes(row.value)}`}
          onclick={() => onselect(row.node)}
        >
          <span class="name">{row.node.name}</span>
          <span class="track">
            <span
              class="fill"
              style="width: {row.pct}%; background: {colorForKey(row.node.name)}"
            ></span>
          </span>
          <span class="value">{formatBytes(row.value)}</span>
        </button>
      </li>
    {/each}
  </ol>
</div>

<style>
  .bars {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    padding: 0.75rem;
    overflow: auto;
  }

  .metric-toggle {
    display: inline-flex;
    align-self: flex-start;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .metric-toggle button {
    padding: 0.25rem 0.6rem;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .metric-toggle button.active {
    background: var(--accent);
    color: #0b0e14;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(6rem, 14rem) 1fr auto;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.25rem 0.35rem;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
  }

  .row:hover {
    background: var(--panel-2);
  }

  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track {
    height: 14px;
    overflow: hidden;
    border-radius: 4px;
    background: color-mix(in srgb, var(--border) 60%, transparent);
  }

  .fill {
    display: block;
    height: 100%;
    min-width: 2px;
    border-radius: 4px;
  }

  .value {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
