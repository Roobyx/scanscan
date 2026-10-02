<script lang="ts">
  import type { HistogramBucket } from "@scanscan/api-types";

  import { colorForKey } from "../lib/color.js";
  import { formatBytes } from "../lib/format.js";

  interface Props {
    items: HistogramBucket[];
    title: string;
  }

  let { items, title }: Props = $props();

  const max = $derived(items.reduce((acc, bucket) => Math.max(acc, bucket.size), 0) || 1);
</script>

<div class="breakdown">
  <h3>{title}</h3>
  <ul>
    {#each items as bucket (bucket.key)}
      <li>
        <span class="label" title={bucket.label}>{bucket.label}</span>
        <span class="track">
          <span
            class="fill"
            style="width: {(bucket.size / max) * 100}%; background: {colorForKey(bucket.key)}"
          ></span>
        </span>
        <span class="count">{bucket.count.toLocaleString()}</span>
        <span class="size">{formatBytes(bucket.size)}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .breakdown {
    flex: 1;
    min-height: 0;
    padding: 0.75rem;
    overflow: auto;
  }

  h3 {
    margin: 0 0 0.6rem;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: minmax(5rem, 12rem) 1fr auto auto;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.78rem;
  }

  .label {
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

  .count,
  .size {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .size {
    min-width: 4.5rem;
    text-align: right;
  }
</style>
