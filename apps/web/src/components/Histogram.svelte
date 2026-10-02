<script lang="ts">
  import type { HistogramBucket } from "@scanscan/api-types";

  import { formatBytes } from "../lib/format.js";

  interface Props {
    items: HistogramBucket[];
  }

  let { items }: Props = $props();

  const max = $derived(items.reduce((acc, bucket) => Math.max(acc, bucket.size), 0) || 1);
</script>

<div class="histogram" role="group" aria-label="Size histogram">
  {#each items as bucket (bucket.key)}
    <div
      class="col"
      title={`${bucket.label}: ${formatBytes(bucket.size)} · ${bucket.count.toLocaleString()} items`}
    >
      <div class="bar-wrap">
        <div class="bar" style="height: {(bucket.size / max) * 100}%"></div>
      </div>
      <span class="label">{bucket.label}</span>
    </div>
  {/each}
</div>

<style>
  .histogram {
    display: flex;
    flex: 1;
    align-items: stretch;
    gap: 0.4rem;
    min-height: 0;
    padding: 1rem 0.75rem 0.75rem;
    overflow: auto;
  }

  .col {
    display: flex;
    flex: 1 1 0;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 2.5rem;
  }

  .bar-wrap {
    display: flex;
    flex: 1;
    align-items: flex-end;
    min-height: 0;
    border-bottom: 1px solid var(--border);
  }

  .bar {
    width: 100%;
    min-height: 2px;
    border-radius: 3px 3px 0 0;
    background: linear-gradient(180deg, var(--accent), color-mix(in srgb, var(--accent) 55%, transparent));
  }

  .label {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.68rem;
    text-align: center;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
