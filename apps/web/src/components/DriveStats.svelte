<script lang="ts">
  import type { DriveInfo } from "@scanscan/api-types";

  import { formatBytes, percent } from "../lib/format.js";

  interface Props {
    drive: DriveInfo | null;
  }

  let { drive }: Props = $props();

  const total = $derived.by(() => drive?.totalBytes ?? 0);
  const free = $derived.by(() => drive?.freeBytes ?? 0);
  const used = $derived.by(() => drive?.usedBytes ?? Math.max(0, total - free));
  const usedPct = $derived.by(() => percent(used, total));
  const label = $derived.by(() =>
    total > 0
      ? `${formatBytes(free)} free of ${formatBytes(total)} · ${usedPct.toFixed(0)}% used`
      : "",
  );
  const detail = $derived.by(() => {
    const entry = drive;
    if (!entry) return "";
    return `${entry.path} (${entry.containerPath}) · ${formatBytes(used)} used · ${formatBytes(free)} free of ${formatBytes(total)}`;
  });
</script>

{#if drive && total > 0}
  <div class="stats" title={detail}>
    <span class="meter">
      <span
        class="fill"
        class:tight={usedPct >= 90}
        class:critical={usedPct >= 97}
        style="width: {usedPct}%"
      ></span>
    </span>
    <span class="text">{label}</span>
  </div>
{/if}

<style>
  .stats {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .meter {
    display: block;
    width: 72px;
    height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--border) 70%, transparent);
  }

  .meter .fill {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }

  .meter .fill.tight {
    background: var(--warn);
  }

  .meter .fill.critical {
    background: var(--err);
  }

  .text {
    color: var(--muted);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
