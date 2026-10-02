<script lang="ts">
  import type { DupGroup } from "../lib/api.js";
  import { errorMessage, getDuplicates } from "../lib/api.js";
  import { formatBytes } from "../lib/format.js";

  interface Props {
    scanId: string;
    scope: number;
  }

  let { scanId, scope }: Props = $props();

  type Mode = "name+size" | "name+size+mtime";

  let mode = $state<Mode>("name+size");
  let groups = $state<DupGroup[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let expanded = $state<number | null>(null);
  let seq = 0;

  const totalWasted = $derived(groups.reduce((sum, group) => sum + group.wasted, 0));

  async function load(id: string, scopeId: number, dupMode: Mode): Promise<void> {
    const ticket = ++seq;
    loading = true;
    error = null;
    try {
      const response = await getDuplicates(id, { scope: scopeId, mode: dupMode, limit: 200 });
      if (ticket !== seq) return;
      groups = response.groups;
      expanded = null;
    } catch (cause) {
      if (ticket !== seq) return;
      error = errorMessage(cause);
      groups = [];
    } finally {
      if (ticket === seq) loading = false;
    }
  }

  $effect(() => {
    void load(scanId, scope, mode);
  });

  function toggle(index: number): void {
    expanded = expanded === index ? null : index;
  }
</script>

<div class="duplicates">
  <div class="toolbar">
    <div class="toggle" role="group" aria-label="Duplicate match mode">
      <button
        type="button"
        class:active={mode === "name+size"}
        onclick={() => (mode = "name+size")}
      >
        name+size
      </button>
      <button
        type="button"
        class:active={mode === "name+size+mtime"}
        onclick={() => (mode = "name+size+mtime")}
      >
        name+size+mtime
      </button>
    </div>
    <span class="muted summary">
      {groups.length.toLocaleString()} groups · {formatBytes(totalWasted)} wasted
    </span>
  </div>

  {#if loading && groups.length === 0}
    <p class="state">Scanning for duplicates…</p>
  {:else if error}
    <p class="state err" role="alert">{error}</p>
  {:else if groups.length === 0}
    <p class="state">No duplicate files found in this scope.</p>
  {:else}
    <ul class="groups">
      {#each groups as group, index (index)}
        <li class="group">
          <button
            type="button"
            class="group-head"
            aria-expanded={expanded === index}
            onclick={() => toggle(index)}
          >
            <span class="caret" aria-hidden="true">{expanded === index ? "▾" : "▸"}</span>
            <span class="key" title={group.key}>{group.key}</span>
            <span class="count">{group.count.toLocaleString()}×</span>
            <span class="size">{formatBytes(group.size)}</span>
            <span class="wasted">{formatBytes(group.wasted)} wasted</span>
          </button>
          {#if expanded === index}
            <ul class="items">
              {#each group.items as item (item.id)}
                <li>
                  <span class="item-name" title={item.name}>{item.name}</span>
                  <span class="item-id">#{item.id}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .duplicates {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .toggle {
    display: inline-flex;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .toggle button {
    padding: 0.3rem 0.6rem;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .toggle button.active {
    background: var(--accent);
    color: #0b0e14;
  }

  .summary {
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0.75rem;
    list-style: none;
  }

  .group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    overflow: hidden;
  }

  .group-head {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto auto;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.5rem 0.6rem;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
  }

  .group-head:hover {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .caret {
    color: var(--muted);
    font-size: 0.7rem;
  }

  .key {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count,
  .size,
  .wasted {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .count {
    color: var(--muted);
  }

  .size {
    color: var(--text);
  }

  .wasted {
    color: var(--warn);
  }

  .items {
    margin: 0;
    padding: 0.25rem 0.6rem 0.5rem 1.8rem;
    border-top: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
    list-style: none;
  }

  .items li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.2rem 0;
    font-size: 0.78rem;
  }

  .item-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-id {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .state {
    margin: 2rem auto;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }

  .muted {
    color: var(--muted);
  }
</style>
