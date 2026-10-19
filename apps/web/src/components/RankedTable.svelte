<script lang="ts">
  import type { NodeRecord } from "@scanscan/api-types";

  import { errorMessage, getChildren } from "../lib/api.js";
  import { colorForExtension, extensionOf } from "../lib/color.js";
  import { formatBytes } from "../lib/format.js";

  import NodeIcon from "./NodeIcon.svelte";
  import Spinner from "./Spinner.svelte";

  interface RootInfo {
    id: number;
    name: string;
    size: number;
    items: number;
  }

  interface Props {
    scanId: string | null;
    /** The snapshot's starting root: the tree is always shown from here. */
    root: RootInfo | null;
    /** Node ids from the root down to the current scope. */
    path: number[];
    scope: number;
    onselect: (node: NodeRecord) => void;
  }

  interface Row {
    node: NodeRecord;
    depth: number;
  }

  interface Load {
    id: number;
    items: NodeRecord[];
  }

  let { scanId, root, path, scope, onselect }: Props = $props();

  type SortKey = "name" | "sizeAlloc" | "sizeApparent" | "children";

  const ROW_HEIGHT = 28;
  const OVERSCAN = 8;
  const VIRTUAL_THRESHOLD = 200;
  const INDENT = 14;

  let sortKey = $state<SortKey>("sizeAlloc");
  let sortDir = $state<"asc" | "desc">("desc");
  let loads = $state<Load[]>([]);
  let expanded = $state<number[]>([]);
  let busing = $state<number[]>([]);
  let error = $state<string | null>(null);

  let wrap = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewport = $state(0);

  /** The starting root as a row, so the tree always shows where the scan began. */
  function rootNode(): NodeRecord | null {
    const start = root;
    if (!start) return null;
    return {
      id: start.id,
      name: start.name,
      kind: "directory",
      sizeApparent: start.size,
      sizeAlloc: start.size,
      mtimeMs: 0,
      subtreeSize: start.size,
      children: start.items,
      hasChildren: true,
    };
  }

  function itemsOf(id: number): NodeRecord[] | null {
    const found = loads.find((entry) => entry.id === id);
    return found ? found.items : null;
  }

  function isExpanded(id: number): boolean {
    return expanded.includes(id);
  }

  async function loadChildren(nodeId: number): Promise<void> {
    const id = scanId;
    if (!id || itemsOf(nodeId) || busing.includes(nodeId)) return;
    busing = [...busing, nodeId];
    error = null;
    try {
      const response = await getChildren(id, nodeId, { sort: "size", limit: 1000 });
      loads = [...loads, { id: nodeId, items: response.items }];
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busing = busing.filter((entry) => entry !== nodeId);
    }
  }

  function order(items: NodeRecord[]): NodeRecord[] {
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
  }

  function walk(out: Row[], node: NodeRecord, depth: number): void {
    out.push({ node, depth });
    if (node.kind !== "directory" || !isExpanded(node.id)) return;
    const children = itemsOf(node.id);
    if (!children) return;
    for (const child of order(children)) walk(out, child, depth + 1);
  }

  const rows = $derived.by(() => {
    const out: Row[] = [];
    const start = rootNode();
    if (start) walk(out, start, 0);
    return out;
  });

  const virtual = $derived(rows.length > VIRTUAL_THRESHOLD);
  const from = $derived(
    virtual
      ? Math.min(rows.length, Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN))
      : 0,
  );
  const to = $derived(
    virtual
      ? Math.min(rows.length, Math.ceil((scrollTop + viewport) / ROW_HEIGHT) + OVERSCAN)
      : rows.length,
  );
  const visible = $derived(rows.slice(from, to));
  const padTop = $derived(from * ROW_HEIGHT);
  const padBottom = $derived(Math.max(0, (rows.length - to) * ROW_HEIGHT));
  const hasRoot = $derived(root !== null);

  function toggleSort(key: SortKey): void {
    if (sortKey === key) {
      sortDir = sortDir === "asc" ? "desc" : "asc";
      return;
    }
    sortKey = key;
    sortDir = key === "name" ? "asc" : "desc";
  }

  function arrow(key: SortKey): string {
    if (sortKey !== key) return "";
    return sortDir === "asc" ? " ▲" : " ▼";
  }

  function toggles(node: NodeRecord): void {
    if (node.kind !== "directory") return;
    if (isExpanded(node.id)) {
      expanded = expanded.filter((entry) => entry !== node.id);
      return;
    }
    expanded = [...expanded, node.id];
    void loadChildren(node.id);
  }

  function activate(node: NodeRecord): void {
    onselect(node);
    if (node.kind === "directory" && !isExpanded(node.id)) {
      expanded = [...expanded, node.id];
      void loadChildren(node.id);
    }
  }

  function indent(depth: number): string {
    return `padding-left: ${depth * INDENT + 4}px`;
  }

  function tintOf(node: NodeRecord): string {
    if (node.kind !== "file") return "var(--muted)";
    return colorForExtension(extensionOf(node.name));
  }

  function titleOf(node: NodeRecord): string {
    const tag = node.label ? ` · ${node.label}` : node.dockerMount ? " · docker" : "";
    return `${node.name}${tag}${node.error ? " · error" : ""}`;
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
    const scan = scanId;
    const start = root;
    const chain = path;
    if (!scan || !start) return;
    const missing: number[] = [];
    for (const id of [start.id, ...chain]) {
      if (missing.includes(id) || isExpanded(id)) continue;
      missing.push(id);
    }
    if (missing.length > 0) expanded = [...expanded, ...missing];
    for (const id of [start.id, ...chain]) void loadChildren(id);
  });

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
  {#if busing.length > 0}
    <p class="busy" role="status" aria-label="Loading folders">
      <Spinner size={12} label="Loading folders" />
      <span>Loading {busing.length} folder{busing.length > 1 ? "s" : ""}…</span>
    </p>
  {/if}
  <table>
    <thead>
      <tr>
        <th class="chev"></th>
        <th class="icon"></th>
        <th class="name">
          <button type="button" onclick={() => toggleSort("name")}>Name{arrow("name")}</button>
        </th>
        <th class="num">
          <button type="button" onclick={() => toggleSort("sizeAlloc")}>
            Allocated{arrow("sizeAlloc")}
          </button>
        </th>
        <th class="num">
          <button type="button" onclick={() => toggleSort("sizeApparent")}>
            Apparent{arrow("sizeApparent")}
          </button>
        </th>
        <th class="num">
          <button type="button" onclick={() => toggleSort("children")}>
            Items{arrow("children")}
          </button>
        </th>
      </tr>
    </thead>
    <tbody>
      {#if virtual && padTop > 0}
        <tr class="spacer" aria-hidden="true">
          <td colspan="6" style="height: {padTop}px"></td>
        </tr>
      {/if}
      {#each visible as row (row.node.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions a11y_no_noninteractive_tabindex -->
        <tr
          class="row"
          class:current={row.node.id === scope}
          class:loading={busing.includes(row.node.id)}
        >
          <td class="chev">
            {#if busing.includes(row.node.id)}
              <Spinner size={12} label="Loading folder" />
            {:else if row.node.kind === "directory"}
              <button
                type="button"
                class="chev-button"
                aria-label={isExpanded(row.node.id) ? "Collapse" : "Expand"}
                onclick={() => toggles(row.node)}
              >
                {isExpanded(row.node.id) ? "▼" : "▶"}
              </button>
            {/if}
          </td>
          <td class="icon">
            <NodeIcon kind={row.node.kind} color={tintOf(row.node)} />
          </td>
          <td class="name" style={indent(row.depth)}>
            <button
              type="button"
              class="link"
              title={titleOf(row.node)}
              onclick={() => activate(row.node)}
              onkeydown={(event) => handleRowKeydown(event, row.node)}
            >
              {row.node.name}
            </button>
          </td>
          <td class="num">{formatBytes(row.node.sizeAlloc)}</td>
          <td class="num">{formatBytes(row.node.sizeApparent)}</td>
          <td class="num">
            {row.node.kind === "directory" ? row.node.children.toLocaleString() : "—"}
          </td>
        </tr>
      {/each}
      {#if virtual && padBottom > 0}
        <tr class="spacer" aria-hidden="true">
          <td colspan="6" style="height: {padBottom}px"></td>
        </tr>
      {/if}
    </tbody>
  </table>
  {#if error}
    <p class="empty err" role="alert">{error}</p>
  {/if}
  {#if !hasRoot}
    <p class="empty">Select a snapshot to see its tree.</p>
  {/if}
  {#if hasRoot && rows.length === 0}
    <p class="empty">No items in this snapshot.</p>
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

  thead th.chev {
    width: 22px;
  }

  thead th.icon {
    width: 22px;
  }

  thead th.name {
    width: 46%;
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

  tbody tr.row:hover {
    background: var(--panel-2);
  }

  tbody tr.row.current {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  tbody tr.row.loading {
    background: color-mix(in srgb, var(--accent) 7%, transparent);
  }

  .busy {
    position: sticky;
    top: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0;
    padding: 0.35rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: var(--panel-2);
    color: var(--muted);
    font-size: 0.75rem;
  }

  td {
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  td.chev {
    padding: 0 0.1rem;
    text-align: center;
  }

  td.icon {
    padding: 0 0.2rem;
  }

  .chev-button {
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.6rem;
    cursor: pointer;
  }

  .chev-button:hover {
    background: var(--border);
    color: var(--text);
  }

  .chev-button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .link {
    display: block;
    width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .link:hover {
    color: var(--accent);
  }

  .link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
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

  .wrap.virtual td.chev {
    padding: 0 0.1rem;
  }

  .wrap.virtual td.icon {
    padding: 0 0.2rem;
  }

  .wrap.virtual tr.spacer td {
    padding: 0;
    border: none;
  }

  .empty {
    padding: 1rem;
    color: var(--muted);
    text-align: center;
  }

  .empty.err {
    color: var(--err);
  }
</style>
