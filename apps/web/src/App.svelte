<script lang="ts">
  import type { HealthResponse, NodeRecord, ScanSummary, Tile } from "@scanscan/api-types";
  import { onMount } from "svelte";

  import Breadcrumb from "./components/Breadcrumb.svelte";
  import RankedTable from "./components/RankedTable.svelte";
  import ScanLauncher from "./components/ScanLauncher.svelte";
  import Treemap from "./components/Treemap.svelte";
  import { errorMessage, getChildren, getHealth, getScan, getTiles, listScans } from "./lib/api.js";
  import type { ColorMode } from "./lib/color.js";
  import { formatBytes } from "./lib/format.js";

  interface Crumb {
    id: number;
    name: string;
  }

  let health = $state<HealthResponse | null>(null);
  let healthError = $state<string | null>(null);
  let scans = $state<ScanSummary[]>([]);
  let selectedId = $state<string | null>(null);

  let scopePath = $state<Crumb[]>([]);
  let scopeId = $state(0);

  let tiles = $state<Tile[]>([]);
  let children = $state<NodeRecord[]>([]);
  let childTotal = $state(0);
  let truncated = $state(false);

  let colorMode = $state<ColorMode>("size");
  let loading = $state(false);
  let error = $state<string | null>(null);

  let treemapHost = $state<HTMLDivElement | null>(null);
  let hostW = $state(0);
  let hostH = $state(0);
  let requestSeq = 0;

  function bucket(value: number): number {
    if (value <= 0) return 0;
    return Math.max(256, Math.round(value / 128) * 128);
  }

  const tileW = $derived(bucket(hostW));
  const tileH = $derived(bucket(hostH));
  const scopeLabel = $derived(scopePath[scopePath.length - 1]?.name ?? "/");
  const totalBytes = $derived(tiles.reduce((sum, tile) => sum + tile.size, 0));

  async function loadScans(): Promise<void> {
    try {
      scans = await listScans();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function selectScan(id: string): Promise<void> {
    selectedId = id;
    scopeId = 0;
    let summary = scans.find((scan) => scan.id === id) ?? null;
    if (!summary) {
      try {
        summary = await getScan(id);
      } catch (cause) {
        error = errorMessage(cause);
      }
    }
    scopePath = [{ id: 0, name: summary?.roots[0] ?? "/" }];
  }

  async function handleCreated(id: string): Promise<void> {
    await loadScans();
    await selectScan(id);
  }

  async function loadScope(
    scanId: string,
    scope: number,
    width: number,
    height: number,
    color: ColorMode,
  ): Promise<void> {
    const seq = requestSeq + 1;
    requestSeq = seq;
    loading = true;
    error = null;
    try {
      const [tilesResponse, childrenResponse] = await Promise.all([
        getTiles(scanId, { scope, depth: 2, w: width, h: height, color }),
        getChildren(scanId, scope, { sort: "size", limit: 200 }),
      ]);
      if (seq !== requestSeq) return;
      tiles = tilesResponse.tiles;
      truncated = tilesResponse.truncated;
      children = childrenResponse.items;
      childTotal = childrenResponse.total;
      const root = scopePath[0];
      if (scopePath.length === 1 && root && root.id !== tilesResponse.scope) {
        scopePath = [{ id: tilesResponse.scope, name: root.name }];
      }
    } catch (cause) {
      if (seq !== requestSeq) return;
      error = errorMessage(cause);
    } finally {
      if (seq === requestSeq) loading = false;
    }
  }

  function pushScope(id: number, name: string): void {
    scopePath = [...scopePath, { id, name }];
    scopeId = id;
  }

  async function openTile(tile: Tile): Promise<void> {
    const scanId = selectedId;
    if (!scanId) return;
    try {
      const response = await getChildren(scanId, tile.node, { limit: 1 });
      if (response.total === 0) return;
      pushScope(tile.node, tile.name);
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  function openNode(node: NodeRecord): void {
    if (node.kind !== "directory") return;
    if (node.children <= 0 && node.hasChildren !== true) return;
    pushScope(node.id, node.name);
  }

  function navigate(index: number): void {
    const target = scopePath[index];
    if (!target) return;
    scopePath = scopePath.slice(0, index + 1);
    scopeId = target.id;
  }

  function handleScanChange(event: Event): void {
    const value = (event.currentTarget as HTMLSelectElement).value;
    if (value.length > 0) void selectScan(value);
  }

  onMount(() => {
    void (async () => {
      try {
        health = await getHealth();
      } catch (cause) {
        healthError = errorMessage(cause);
      }
      await loadScans();
      const completed = scans
        .filter((scan) => scan.state === "completed")
        .sort((a, b) => (b.finishedAtMs ?? b.startedAtMs) - (a.finishedAtMs ?? a.startedAtMs));
      const newest = completed[0];
      if (newest) await selectScan(newest.id);
    })();
  });

  $effect(() => {
    const host = treemapHost;
    if (!host) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      hostW = entry.contentRect.width;
      hostH = entry.contentRect.height;
    });
    observer.observe(host);
    return () => observer.disconnect();
  });

  $effect(() => {
    const scanId = selectedId;
    const scope = scopeId;
    const width = tileW;
    const height = tileH;
    const color = colorMode;
    if (!scanId || width <= 0 || height <= 0) return;
    void loadScope(scanId, scope, width, height, color);
  });
</script>

<div class="shell">
  <header>
    <div class="brand">
      <span class="logo">▚</span>
      <h1>scanscan</h1>
    </div>
    <div class="controls">
      {#if scans.length > 0}
        <select value={selectedId ?? ""} onchange={handleScanChange} aria-label="Scan">
          <option value="" disabled>Select scan</option>
          {#each scans as scan (scan.id)}
            <option value={scan.id}>{scan.roots.join(", ")} · {scan.state}</option>
          {/each}
        </select>
      {/if}
      <div class="toggle" role="group" aria-label="Color mode">
        <button type="button" class:active={colorMode === "size"} onclick={() => (colorMode = "size")}>
          Size
        </button>
        <button type="button" class:active={colorMode === "ext"} onclick={() => (colorMode = "ext")}>
          Extension
        </button>
      </div>
      <div class="status">
        {#if healthError}
          <span class="pill err" title={healthError}>API unreachable</span>
        {:else if health}
          <span class="pill ok">API {health.status}</span>
          <span class="pill" class:ok={health.core.connected} class:muted={!health.core.connected}>
            core {health.core.connected ? "connected" : "idle"}
          </span>
        {:else}
          <span class="pill muted">connecting…</span>
        {/if}
      </div>
    </div>
  </header>

  {#if !selectedId}
    <main class="empty-main">
      <ScanLauncher oncreated={handleCreated} />
    </main>
  {:else}
    <main>
      <section class="panel">
        <div class="panel-head">
          <div class="crumbs">
            <Breadcrumb path={scopePath} onnavigate={navigate} />
          </div>
          <span class="muted">{formatBytes(totalBytes)}{truncated ? " · truncated" : ""}</span>
        </div>
        <div class="canvas-host" bind:this={treemapHost}>
          {#if loading && tiles.length === 0}
            <p class="state">Loading tiles…</p>
          {:else if error}
            <p class="state err">{error}</p>
          {:else if tiles.length === 0}
            <p class="state">No tiles in {scopeLabel}.</p>
          {:else}
            <Treemap {tiles} {colorMode} onselect={openTile} />
          {/if}
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <h2>Ranked</h2>
          <span class="muted">{childTotal.toLocaleString()} items</span>
        </div>
        {#if loading && children.length === 0}
          <p class="state">Loading…</p>
        {:else}
          <RankedTable items={children} onselect={openNode} />
        {/if}
      </section>
    </main>
  {/if}

  <footer>
    <span>protocol v{health?.protocol ?? "—"}</span>
    <span class="muted">read-only · no telemetry</span>
  </footer>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .logo {
    color: var(--accent);
    font-size: 1.25rem;
  }

  h1 {
    margin: 0;
    font-size: 1rem;
    letter-spacing: 0.02em;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }

  select {
    max-width: 18rem;
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
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

  .status {
    display: flex;
    gap: 0.5rem;
  }

  .pill {
    padding: 0.2rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 0.75rem;
  }

  .pill.ok {
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 40%, transparent);
  }

  .pill.err {
    color: var(--err);
    border-color: color-mix(in srgb, var(--err) 40%, transparent);
  }

  .pill.muted,
  .muted {
    color: var(--muted);
  }

  main {
    flex: 1;
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1rem;
    padding: 1rem;
    min-height: 0;
  }

  main.empty-main {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-height: 2.6rem;
    padding: 0.6rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2 {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .crumbs {
    min-width: 0;
    overflow: hidden;
  }

  .canvas-host {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .state {
    margin: auto;
    padding: 1rem;
    color: var(--muted);
  }

  .state.err {
    color: var(--err);
  }

  footer {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--muted);
  }
</style>
