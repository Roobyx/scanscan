<script lang="ts">
  import type { NodeRecord } from "@scanscan/api-types";

  import { formatBytes } from "../lib/format.js";

  interface Props {
    items: NodeRecord[];
    onselect: (node: NodeRecord) => void;
  }

  let { items, onselect }: Props = $props();

  type SortKey = "name" | "sizeAlloc" | "sizeApparent" | "children" | "kind";

  const ROW_HEIGHT = 28;
  const OVERSCAN = 8;
  const VIRTUAL_THRESHOLD = 200;

  let sortKey = $state<SortKey>("sizeAlloc");
  let sortDir = $state<"asc" | "desc">("desc");

  let wrap = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewport = $state(0);

  const sorted = $derived.by(() => {
    const copy = [...items];
    const direction = sortDir === "asc" ? 1 : -1;
    copy.sort((a, b) => {
      const left = a[sortKey];
      const right = b[sortKey];
      if (typeof left === "number" && typeof right === "number") {
        return (left - right) * direction;
      }
      return String(left).localeCompare(String(right)) * direction;
    });
    return copy;
  });

  const virtual = $derived(sorted.length > VIRTUAL_THRESHOLD);
  const start = $derived(
    virtual
      ? Math.min(sorted.length, Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN))
      : 0,
  );
  const end = $derived(
    virtual
      ? Math.min(sorted.length, Math.ceil((scrollTop + viewport) / ROW_HEIGHT) + OVERSCAN)
      : sorted.length,
  );
  const visible = $derived(sorted.slice(start, end));
  const padTop = $derived(start * ROW_HEIGHT);
  const padBottom = $derived(Math.max(0, (sorted.length - end) * ROW_HEIGHT));

  function toggle(key: SortKey): void {
    if (sortKey === key) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
      return;
    }
    sortKey = key;
    sortDir = key === "name" || key === "kind" ? "asc" : "desc";
  }

  function arrow(key: SortKey): string {
    if (sortKey !== key) return "";
    return sortDir === "asc" ? " ▲" : " ▼";
  }

  function activate(node: NodeRecord): void {
    onselect(node);
  }

  function handleRowKeydown(event: KeyboardEvent, node: NodeRecord): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    activate(node);
  }

  function handleScroll(): void {
    if (wrap) scrollTop = wrap.scrollTop;
  }

  $effect(() => {
    const element = wrap;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      viewport = element.clientHeight;
    });
    observer.observe(element);
    viewport = element.clientHeight;
    return () => observer.disconnect();
  });
</script>

<div class="wrap" class:virtual bind:this={wrap} onscroll={handleScroll}>
  <table>
    <thead>
      <tr>
        <th><button type="button" onclick={() => toggle("name")}>Name{arrow("name")}</button></th>
        <th class="num">
          <button type="button" onclick={() => toggle("sizeAlloc")}>Allocated{arrow("sizeAlloc")}</button>
        </th>
        <th class="num">
          <button type="button" onclick={() => toggle("sizeApparent")}>Apparent{arrow("sizeApparent")}</button>
        </th>
        <th class="num">
          <button type="button" onclick={() => toggle("children")}>Items{arrow("children")}</button>
        </th>
        <th><button type="button" onclick={() => toggle("kind")}>Kind{arrow("kind")}</button></th>
      </tr>
    </thead>
    <tbody>
      {#if virtual && padTop > 0}
        <tr class="spacer" aria-hidden="true">
          <td colspan="5" style="height: {padTop}px"></td>
        </tr>
      {/if}
      {#each visible as node (node.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions a11y_no_noninteractive_tabindex -->
        <tr
          tabindex="0"
          onclick={() => activate(node)}
          onkeydown={(event) => handleRowKeydown(event, node)}
        >
          <td class="name" title={node.name}>{node.name}</td>
          <td class="num">{formatBytes(node.sizeAlloc)}</td>
          <td class="num">{formatBytes(node.sizeApparent)}</td>
          <td class="num">{node.kind === "directory" ? node.children.toLocaleString() : "—"}</td>
          <td class="kind">
            {node.kind}{node.dockerMount ? " · docker" : ""}{node.error ? " · error" : ""}
          </td>
        </tr>
      {/each}
      {#if virtual && padBottom > 0}
        <tr class="spacer" aria-hidden="true">
          <td colspan="5" style="height: {padBottom}px"></td>
        </tr>
      {/if}
    </tbody>
  </table>
  {#if sorted.length === 0}
    <p class="empty">No items in this scope.</p>
  {/if}
</div>

<style>
  .wrap {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  table {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
    text-align: left;
  }

  thead th:first-child {
    width: 42%;
  }

  thead button {
    width: 100%;
    padding: 0.5rem 0.6rem;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.7rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    text-align: inherit;
    cursor: pointer;
  }

  thead button:hover {
    color: var(--text);
  }

  th.num button,
  td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  tbody tr {
    cursor: pointer;
  }

  tbody tr:hover {
    background: var(--panel-2);
  }

  tbody tr:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  td {
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  .wrap.virtual tbody tr {
    height: 28px;
  }

  .wrap.virtual td {
    height: 28px;
    padding: 0 0.6rem;
    line-height: 28px;
    white-space: nowrap;
  }

  .wrap.virtual tr.spacer td {
    padding: 0;
    border: none;
  }

  td.name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  td.kind {
    overflow: hidden;
    color: var(--muted);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    padding: 1rem;
    color: var(--muted);
    text-align: center;
  }
</style>
