<script lang="ts">
  import type { DriveInfo, ScanSummary } from "@scanscan/api-types";
  import { onMount } from "svelte";

  import { formatAgo, formatBytes } from "../lib/format.js";
  import { scanRootLabel } from "../lib/mounts.js";
  import { stateMeta } from "../lib/scan-state.js";

  import Icon from "./Icon.svelte";

  interface Props {
    scans: ScanSummary[];
    selectedId: string | null;
    drives: DriveInfo[];
    disabled?: boolean;
    onselect: (scanId: string) => void;
  }

  let { scans, selectedId, drives, disabled = false, onselect }: Props = $props();

  let open = $state(false);
  let host = $state<HTMLDivElement | null>(null);

  const selected = $derived(scans.find((scan) => scan.id === selectedId) ?? null);

  function choose(scanId: string): void {
    open = false;
    onselect(scanId);
  }

  onMount(() => {
    function handlePointer(event: PointerEvent): void {
      if (!open) return;
      const target = event.target;
      if (host && target instanceof Node && !host.contains(target)) open = false;
    }
    document.addEventListener("pointerdown", handlePointer);
    return () => document.removeEventListener("pointerdown", handlePointer);
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") open = false;
  }}
/>

<div class="switcher" bind:this={host}>
  <button
    type="button"
    class="trigger"
    class:open={open}
    disabled={disabled || scans.length === 0}
    aria-haspopup="listbox"
    aria-expanded={open}
    title={selected ? selected.roots.join(", ") : undefined}
    onclick={() => (open = !open)}
  >
    <span class="leading">
      <Icon name="scan" size={14} />
    </span>
    <span class="label">
      {#if selected}
        {scanRootLabel(selected, drives)}
      {:else if scans.length > 0}
        Select snapshot
      {:else}
        No snapshots
      {/if}
    </span>
    {#if selected}
      {@const meta = stateMeta(selected.state)}
      <span class="state {meta.tone}">
        <Icon name={meta.icon} size={11} />
        {meta.label}
      </span>
    {/if}
    <span class="chevron">
      <Icon name="chevron-down" size={14} />
    </span>
  </button>

  {#if open}
    <ul class="menu" role="listbox" aria-label="Snapshots">
      {#each scans as scan (scan.id)}
        {@const meta = stateMeta(scan.state)}
        <li>
          <button
            type="button"
            class="option"
            class:selected={scan.id === selectedId}
            role="option"
            aria-selected={scan.id === selectedId}
            onclick={() => choose(scan.id)}
          >
            <span class="option-main">
              <span class="option-title">{scanRootLabel(scan, drives)}</span>
              <span class="option-meta" title={scan.roots.join(", ")}>
                {formatBytes(scan.bytesAlloc)} · {formatAgo(scan.finishedAtMs ?? scan.startedAtMs)}
              </span>
            </span>
            <span class="state {meta.tone}">
              <Icon name={meta.icon} size={11} />
              {meta.label}
            </span>
            {#if scan.id === selectedId}
              <span class="tick">
                <Icon name="check" size={13} />
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .switcher {
    position: relative;
    min-width: 0;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    max-width: 22rem;
    padding: 0.42rem 0.6rem 0.42rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
  }

  .trigger:hover:not(:disabled),
  .trigger.open {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  }

  .trigger:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .leading,
  .chevron,
  .tick {
    display: inline-flex;
    flex: none;
  }

  .leading {
    color: var(--muted);
  }

  .label {
    overflow: hidden;
    min-width: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    margin-left: 0.1rem;
    color: var(--muted);
    transition: transform 120ms ease;
  }

  .trigger.open .chevron {
    transform: rotate(180deg);
  }

  .state {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 0.25rem;
    padding: 0.12rem 0.42rem;
    border: 1px solid transparent;
    border-radius: 999px;
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }

  .state.ok {
    border-color: color-mix(in srgb, var(--ok) 38%, transparent);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    color: var(--ok);
  }

  .state.info {
    border-color: color-mix(in srgb, var(--accent) 42%, transparent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
  }

  .state.warn {
    border-color: color-mix(in srgb, var(--warn) 40%, transparent);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
    color: var(--warn);
  }

  .state.err {
    border-color: color-mix(in srgb, var(--err) 40%, transparent);
    background: color-mix(in srgb, var(--err) 14%, transparent);
    color: var(--err);
  }

  .state.muted {
    border-color: var(--border);
    color: var(--muted);
  }

  .menu {
    position: absolute;
    top: calc(100% + 0.35rem);
    left: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    width: min(max(20rem, 100%), calc(100vw - 2rem));
    max-height: 22rem;
    margin: 0;
    padding: 0.35rem;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    list-style: none;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.55rem;
    border: 1px solid transparent;
    border-radius: 7px;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .option:hover {
    background: var(--panel-2);
  }

  .option.selected {
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .option-main {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .option-title {
    overflow: hidden;
    font-size: 0.8rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .option-meta {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tick {
    color: var(--accent);
  }

  @media (max-width: 720px) {
    .trigger {
      max-width: 14rem;
    }

    .menu {
      right: 0;
      left: auto;
      width: min(22rem, 88vw);
    }
  }
</style>
