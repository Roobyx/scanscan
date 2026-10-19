<script lang="ts">
  import type { HistogramBucket } from "@scanscan/api-types";

  import { groupByFileType } from "../lib/file-types.js";
  import { formatBytes, percent } from "../lib/format.js";

  interface Props {
    items: HistogramBucket[];
    /** Currently filtered group id, or null when the whole breakdown shows. */
    selected: string | null;
    onselect: (group: string | null) => void;
  }

  let { items, selected, onselect }: Props = $props();

  const groups = $derived.by(() => groupByFileType(items));
  const total = $derived.by(() => groups.reduce((sum, group) => sum + group.size, 0));

  function share(size: number): number {
    return percent(size, total);
  }

  function toggle(groupId: string): void {
    onselect(selected === groupId ? null : groupId);
  }
</script>

{#if groups.length > 0}
  <div class="file-types">
    <div class="head">
      <h3>Main file types</h3>
      <span class="muted">{formatBytes(total)} in scope</span>
      {#if selected !== null}
        <button type="button" class="link" onclick={() => onselect(null)}>Clear filter</button>
      {/if}
    </div>
    <ul class="groups">
      {#each groups as group (group.id)}
        <li>
          <button
            type="button"
            class="group"
            class:active={selected === group.id}
            aria-pressed={selected === group.id}
            title={`${group.label} · ${group.count.toLocaleString()} files · ${formatBytes(group.size)}`}
            onclick={() => toggle(group.id)}
          >
            <span class="line">
              <span class="dot" style="background: {group.color}"></span>
              <span class="label">{group.label}</span>
              <span class="size">{formatBytes(group.size)}</span>
            </span>
            <span class="track">
              <span class="fill" style="width: {share(group.size)}%; background: {group.color}"></span>
            </span>
            <span class="line">
              <span class="count">{group.count.toLocaleString()} files</span>
              <span class="share">{share(group.size).toFixed(0)}%</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .file-types {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 0.75rem 0;
    border-bottom: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .head h3 {
    margin: 0;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .head .muted {
    flex: 1;
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }

  .link {
    padding: 0.1rem 0.35rem;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
  }

  .link:hover {
    background: var(--panel-2);
  }

  .groups {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .groups li {
    display: flex;
    flex: 1;
    min-width: 8.5rem;
  }

  .group {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.3rem;
    width: 100%;
    padding: 0.45rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
  }

  .group:hover {
    border-color: var(--accent);
  }

  .group.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .group:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .size,
  .share,
  .count {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .count,
  .share {
    color: var(--muted);
    font-size: 0.72rem;
  }

  .share {
    flex: 1;
    text-align: right;
  }

  .track {
    display: block;
    height: 6px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--border) 60%, transparent);
  }

  .fill {
    display: block;
    height: 100%;
    border-radius: 999px;
  }
</style>
