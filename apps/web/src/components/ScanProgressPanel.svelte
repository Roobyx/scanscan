<script lang="ts">
  import { onMount } from "svelte";

  import type { ScanProgress } from "@scanscan/api-types";

  import { getProgress, listScans } from "../lib/api.js";
  import { formatBytes } from "../lib/format.js";

  interface Props {
    onfinished: (scanId: string) => void;
  }

  let { onfinished }: Props = $props();

  let active = $state<ScanProgress[]>([]);
  let known = new Set<string>();

  async function tick(): Promise<void> {
    try {
      const scans = await listScans();
      const running = scans.filter((scan) => scan.state === "running" || scan.state === "queued");
      const runningIds = new Set(running.map((scan) => scan.id));

      for (const id of known) {
        if (!runningIds.has(id)) onfinished(id);
      }
      known = runningIds;

      const next: ScanProgress[] = [];
      for (const scan of running) {
        try {
          next.push(await getProgress(scan.id));
        } catch {
          // Skip a scan whose progress cannot be read this tick.
        }
      }
      active = next;
    } catch {
      // Ignore transient list failures; the next tick retries.
    }
  }

  onMount(() => {
    void tick();
    const timer = setInterval(() => void tick(), 1500);
    return () => clearInterval(timer);
  });
</script>

{#if active.length > 0}
  <aside class="toasts" aria-live="polite" aria-label="Running scans">
    {#each active as scan (scan.scanId)}
      <div class="toast">
        <div class="row">
          <span class="dot"></span>
          <strong>Scanning</strong>
          <span class="state">{scan.state}</span>
        </div>
        <div class="path" title={scan.currentPath ?? ""}>{scan.currentPath ?? "starting…"}</div>
        <div class="stats">
          <span>{scan.files.toLocaleString()} files</span>
          <span>{scan.dirs.toLocaleString()} dirs</span>
          <span>{formatBytes(scan.bytesAlloc)}</span>
        </div>
        <div class="bar"><div class="fill"></div></div>
      </div>
    {/each}
  </aside>
{/if}

<style>
  .toasts {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    width: min(320px, 90vw);
  }

  .toast {
    padding: 0.75rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .state {
    margin-left: auto;
    color: var(--muted);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .path {
    margin: 0.35rem 0 0.4rem;
    color: var(--muted);
    font-size: 0.72rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stats {
    display: flex;
    gap: 0.75rem;
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }

  .bar {
    margin-top: 0.5rem;
    height: 3px;
    border-radius: 2px;
    background: var(--panel-2);
    overflow: hidden;
  }

  .fill {
    width: 40%;
    height: 100%;
    background: var(--accent);
    animation: slide 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  @keyframes slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(250%);
    }
  }
</style>
