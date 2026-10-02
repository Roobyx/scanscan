<script lang="ts">
  import type { NodeKind, NodeRecord } from "@scanscan/api-types";

  import { errorMessage, searchNodes } from "../lib/api.js";
  import { formatBytes } from "../lib/format.js";

  interface Props {
    scanId: string;
    scope: number;
    onselect: (node: NodeRecord) => void;
    q?: string;
  }

  let { scanId, scope, onselect, q = $bindable("") }: Props = $props();

  let ext = $state("");
  let kind = $state<"" | NodeKind>("");
  let sizeMin = $state("");
  let sizeMax = $state("");
  let results = $state<NodeRecord[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let searched = $state(false);

  function toNumber(value: string): number | undefined {
    const trimmed = value.trim();
    if (trimmed.length === 0) return undefined;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : undefined;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    loading = true;
    error = null;
    searched = true;
    try {
      results = await searchNodes(scanId, {
        scope,
        q: q.trim(),
        ext: ext.trim(),
        kind: kind === "" ? undefined : kind,
        size_min: toNumber(sizeMin),
        size_max: toNumber(sizeMax),
        limit: 500,
      });
    } catch (cause) {
      error = errorMessage(cause);
      results = [];
    } finally {
      loading = false;
    }
  }

  function activate(node: NodeRecord): void {
    if (node.kind === "directory") onselect(node);
  }

  function handleRowKeydown(event: KeyboardEvent, node: NodeRecord): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    activate(node);
  }
</script>

<div class="search">
  <form onsubmit={submit}>
    <label>
      <span>Name</span>
      <input type="text" bind:value={q} placeholder="contains…" />
    </label>
    <label>
      <span>Extension</span>
      <input type="text" bind:value={ext} placeholder="log" />
    </label>
    <label>
      <span>Kind</span>
      <select bind:value={kind}>
        <option value="">any</option>
        <option value="file">file</option>
        <option value="directory">directory</option>
        <option value="symlink">symlink</option>
        <option value="special">special</option>
      </select>
    </label>
    <label>
      <span>Min bytes</span>
      <input type="number" min="0" bind:value={sizeMin} placeholder="0" />
    </label>
    <label>
      <span>Max bytes</span>
      <input type="number" min="0" bind:value={sizeMax} placeholder="∞" />
    </label>
    <button type="submit" disabled={loading}>{loading ? "Searching…" : "Search"}</button>
  </form>

  {#if error}
    <p class="state err" role="alert">{error}</p>
  {:else if loading}
    <p class="state">Searching…</p>
  {:else if searched && results.length === 0}
    <p class="state">No matches.</p>
  {:else if results.length > 0}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th class="num">Allocated</th>
            <th class="num">Apparent</th>
            <th>Kind</th>
          </tr>
        </thead>
        <tbody>
          {#each results as node (node.id)}
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions a11y_no_noninteractive_tabindex -->
            <tr
              class:clickable={node.kind === "directory"}
              tabindex={node.kind === "directory" ? 0 : -1}
              onclick={() => activate(node)}
              onkeydown={(event) => handleRowKeydown(event, node)}
            >
              <td class="name" title={node.name}>{node.name}</td>
              <td class="num">{formatBytes(node.sizeAlloc)}</td>
              <td class="num">{formatBytes(node.sizeApparent)}</td>
              <td class="kind">{node.kind}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .search {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }

  form {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(7rem, 1fr)) auto;
    gap: 0.5rem;
    align-items: end;
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  label span {
    color: var(--muted);
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  input,
  select {
    min-width: 0;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
  }

  input:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  form button {
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--accent);
    color: #0b0e14;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  form button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .table-wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  th {
    position: sticky;
    top: 0;
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
    color: var(--muted);
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    padding: 0.35rem 0.6rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  tr.clickable {
    cursor: pointer;
  }

  tr.clickable:hover {
    background: var(--panel-2);
  }

  tr:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  td.name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  td.kind {
    color: var(--muted);
  }

  th.num,
  td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .state {
    margin: 1rem;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }
</style>
