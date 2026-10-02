<script lang="ts">
  import type {
    HealthResponse,
    HeatmapResponse,
    HierarchyNode,
    HistogramBucket,
    NodeRecord,
    ScanSummary,
    Tile,
  } from "@scanscan/api-types";
  import { onMount } from "svelte";

  import Bars from "./components/Bars.svelte";
  import Breadcrumb from "./components/Breadcrumb.svelte";
  import Breakdown from "./components/Breakdown.svelte";
  import Bubble from "./components/Bubble.svelte";
  import DiffPanel from "./components/DiffPanel.svelte";
  import DockerPanel from "./components/DockerPanel.svelte";
  import DuplicatesPanel from "./components/DuplicatesPanel.svelte";
  import Heatmap from "./components/Heatmap.svelte";
  import Histogram from "./components/Histogram.svelte";
  import Icicle from "./components/Icicle.svelte";
  import RankedTable from "./components/RankedTable.svelte";
  import ScanLauncher from "./components/ScanLauncher.svelte";
  import ScanProgressPanel from "./components/ScanProgressPanel.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SchedulesPanel from "./components/SchedulesPanel.svelte";
  import Sunburst from "./components/Sunburst.svelte";
  import Treemap from "./components/Treemap.svelte";
  import {
    errorMessage,
    exportUrl,
    getChildren,
    getHealth,
    getHeatmap,
    getHistogram,
    getScan,
    getTiles,
    getTop,
    getTree,
    listScans,
    runGc,
  } from "./lib/api.js";
  import type { HistogramDim } from "./lib/api.js";
  import type { ColorMode } from "./lib/color.js";
  import { formatBytes } from "./lib/format.js";

  type ViewId =
    | "treemap"
    | "sunburst"
    | "icicle"
    | "bubble"
    | "bars"
    | "histogram"
    | "heatmap"
    | "extensions"
    | "age"
    | "owners"
    | "docker"
    | "duplicates"
    | "diff"
    | "schedules";

  interface ViewDef {
    id: ViewId;
    label: string;
  }

  const VIEWS: ViewDef[] = [
    { id: "treemap", label: "Treemap" },
    { id: "sunburst", label: "Sunburst" },
    { id: "icicle", label: "Icicle" },
    { id: "bubble", label: "Bubble" },
    { id: "bars", label: "Bars" },
    { id: "histogram", label: "Histogram" },
    { id: "heatmap", label: "Heatmap" },
    { id: "extensions", label: "Extensions" },
    { id: "age", label: "Age" },
    { id: "owners", label: "Owners" },
    { id: "docker", label: "Docker" },
    { id: "duplicates", label: "Duplicates" },
    { id: "diff", label: "Diff" },
    { id: "schedules", label: "Schedules" },
  ];

  interface Crumb {
    id: number;
    name: string;
  }

  interface HashState {
    view: ViewId;
    scope: number | null;
    color: ColorMode;
    q: string;
  }

  function parseHash(): HashState {
    const raw =
      typeof window !== "undefined" && window.location.hash.length > 1
        ? window.location.hash.slice(1)
        : "";
    const params = new URLSearchParams(raw);
    const viewParam = params.get("view");
    const matched = VIEWS.find((entry) => entry.id === viewParam);
    const scopeRaw = params.get("scope");
    const scopeNum = scopeRaw === null ? Number.NaN : Number(scopeRaw);
    return {
      view: matched ? matched.id : "treemap",
      scope: Number.isFinite(scopeNum) && scopeNum >= 0 ? scopeNum : null,
      color: params.get("color") === "ext" ? "ext" : "size",
      q: params.get("q") ?? "",
    };
  }

  const initial = parseHash();
  let pendingScope: number | null = initial.scope;

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

  let colorMode = $state<ColorMode>(initial.color);
  let view = $state<ViewId>(initial.view);
  let loading = $state(false);
  let error = $state<string | null>(null);

  let tree = $state<HierarchyNode | null>(null);
  let topNodes = $state<NodeRecord[]>([]);
  let histogram = $state<HistogramBucket[]>([]);
  let heatmapData = $state<HeatmapResponse | null>(null);
  let viewLoading = $state(false);
  let viewError = $state<string | null>(null);

  let searchOpen = $state(initial.q.length > 0);
  let searchQuery = $state(initial.q);

  let gcBusy = $state(false);
  let gcMessage = $state<string | null>(null);

  let treemapHost = $state<HTMLDivElement | null>(null);
  let hostW = $state(0);
  let hostH = $state(0);
  let tilesSeq = 0;
  let childrenSeq = 0;
  let viewSeq = 0;

  function bucket(value: number): number {
    if (value <= 0) return 0;
    return Math.max(256, Math.round(value / 128) * 128);
  }

  const tileW = $derived(bucket(hostW));
  const tileH = $derived(bucket(hostH));
  const scopeLabel = $derived(scopePath[scopePath.length - 1]?.name ?? "/");
  const totalBytes = $derived(tiles.reduce((sum, tile) => sum + tile.size, 0));
  const isHierarchyView = $derived(view === "sunburst" || view === "icicle" || view === "bubble");
  const hasCompleted = $derived(scans.some((scan) => scan.state === "completed"));

  const viewSummary = $derived.by(() => {
    if (view === "treemap") return `${formatBytes(totalBytes)}${truncated ? " · truncated" : ""}`;
    if (isHierarchyView) return tree ? formatBytes(tree.size) : "—";
    if (view === "bars") return `${topNodes.length} nodes`;
    if (view === "docker") return "containers";
    if (view === "duplicates") return "duplicate files";
    if (view === "diff") return "snapshot diff";
    if (view === "schedules") return "cron jobs";
    return `${histogram.length} buckets`;
  });

  const breakdownTitle = $derived(
    view === "extensions" ? "Extensions" : view === "age" ? "Age" : "Owners",
  );

  async function loadScans(): Promise<void> {
    try {
      scans = await listScans();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function selectScan(id: string): Promise<void> {
    selectedId = id;
    const requested = pendingScope;
    pendingScope = null;
    scopeId = requested ?? 0;
    let summary = scans.find((scan) => scan.id === id) ?? null;
    if (!summary) {
      try {
        summary = await getScan(id);
      } catch (cause) {
        error = errorMessage(cause);
      }
    }
    scopePath = [{ id: requested ?? 0, name: summary?.roots[0] ?? "/" }];
  }

  async function handleCreated(id: string): Promise<void> {
    // A scan was started: refresh the list so it appears, and pick the newest
    // completed snapshot if nothing is selected. Progress is shown globally by
    // <ScanProgressPanel />, and we navigate when it finishes.
    await loadScans();
    if (!selectedId) {
      const newest = scans
        .filter((scan) => scan.state === "completed")
        .sort((a, b) => (b.finishedAtMs ?? b.startedAtMs) - (a.finishedAtMs ?? a.startedAtMs))[0];
      if (newest) await selectScan(newest.id);
    }
    void id;
  }

  async function handleScanFinished(id: string): Promise<void> {
    await loadScans();
    const scan = scans.find((entry) => entry.id === id);
    if (scan?.state === "completed") {
      await selectScan(id);
    }
  }

  async function loadTiles(
    scanId: string,
    scope: number,
    width: number,
    height: number,
    color: ColorMode,
  ): Promise<void> {
    const seq = tilesSeq + 1;
    tilesSeq = seq;
    loading = true;
    error = null;
    try {
      const response = await getTiles(scanId, { scope, depth: 2, w: width, h: height, color });
      if (seq !== tilesSeq) return;
      tiles = response.tiles;
      truncated = response.truncated;
      const root = scopePath[0];
      if (scopePath.length === 1 && root && root.id !== response.scope) {
        scopePath = [{ id: response.scope, name: root.name }];
      }
    } catch (cause) {
      if (seq !== tilesSeq) return;
      error = errorMessage(cause);
    } finally {
      if (seq === tilesSeq) loading = false;
    }
  }

  async function loadChildren(scanId: string, scope: number): Promise<void> {
    const seq = childrenSeq + 1;
    childrenSeq = seq;
    try {
      const response = await getChildren(scanId, scope, { sort: "size", limit: 1000 });
      if (seq !== childrenSeq) return;
      children = response.items;
      childTotal = response.total;
    } catch (cause) {
      if (seq !== childrenSeq) return;
      error = errorMessage(cause);
    }
  }

  async function loadViewData(): Promise<void> {
    const scanId = selectedId;
    if (!scanId) return;
    const seq = viewSeq + 1;
    viewSeq = seq;
    viewError = null;
    if (isHierarchyView) {
      tree = null;
    } else if (view === "bars") {
      topNodes = [];
    } else if (view === "heatmap") {
      heatmapData = null;
    } else {
      histogram = [];
    }
    viewLoading = true;
    try {
      if (isHierarchyView) {
        const response = await getTree(scanId, scopeId, 3, 2000);
        if (seq !== viewSeq) return;
        tree = response.root;
      } else if (view === "bars") {
        const nodes = await getTop(scanId, { n: 25, scope: scopeId });
        if (seq !== viewSeq) return;
        topNodes = nodes;
      } else if (view === "heatmap") {
        const response = await getHeatmap(scanId, scopeId);
        if (seq !== viewSeq) return;
        heatmapData = response;
      } else {
        const dim: HistogramDim =
          view === "histogram" ? "size" : view === "age" ? "age" : view === "owners" ? "owner" : "ext";
        const response = await getHistogram(scanId, dim, scopeId);
        if (seq !== viewSeq) return;
        histogram = response.items;
      }
    } catch (cause) {
      if (seq !== viewSeq) return;
      viewError = errorMessage(cause);
    } finally {
      if (seq === viewSeq) viewLoading = false;
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

  async function openHierarchy(node: HierarchyNode): Promise<void> {
    if (node.kind !== "directory") return;
    if (node.children && node.children.length > 0) {
      pushScope(node.id, node.name);
      return;
    }
    const scanId = selectedId;
    if (!scanId) return;
    try {
      const response = await getChildren(scanId, node.id, { limit: 1 });
      if (response.total === 0) return;
      pushScope(node.id, node.name);
    } catch (cause) {
      error = errorMessage(cause);
    }
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

  async function handleGc(): Promise<void> {
    if (!window.confirm("Delete all but the 3 most recent snapshots?")) return;
    gcBusy = true;
    gcMessage = null;
    error = null;
    try {
      const result = await runGc(3);
      gcMessage = `removed ${result.removed.length}`;
      await loadScans();
      if (selectedId && !scans.some((scan) => scan.id === selectedId)) {
        const completed = scans
          .filter((scan) => scan.state === "completed")
          .sort((a, b) => (b.finishedAtMs ?? b.startedAtMs) - (a.finishedAtMs ?? a.startedAtMs));
        const newest = completed[0];
        if (newest) {
          await selectScan(newest.id);
        } else {
          selectedId = null;
        }
      }
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      gcBusy = false;
    }
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
    if (view !== "treemap") return;
    const scanId = selectedId;
    const scope = scopeId;
    const width = tileW;
    const height = tileH;
    const color = colorMode;
    if (!scanId || width <= 0 || height <= 0) return;
    void loadTiles(scanId, scope, width, height, color);
  });

  $effect(() => {
    const scanId = selectedId;
    const scope = scopeId;
    if (!scanId) return;
    void loadChildren(scanId, scope);
  });

  $effect(() => {
    const scanId = selectedId;
    const active = view;
    if (
      !scanId ||
      active === "treemap" ||
      active === "docker" ||
      active === "duplicates" ||
      active === "diff" ||
      active === "schedules"
    )
      return;
    void loadViewData();
  });

  $effect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams();
    params.set("view", view);
    if (scopeId !== 0) params.set("scope", String(scopeId));
    params.set("color", colorMode);
    const trimmed = searchQuery.trim();
    if (trimmed.length > 0) params.set("q", trimmed);
    const hash = `#${params.toString()}`;
    if (window.location.hash !== hash) window.history.replaceState(null, "", hash);
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
      <button type="button" class="action" onclick={() => (selectedId = null)}>+ New scan</button>
      <div class="toggle" role="group" aria-label="Color mode">
        <button type="button" class:active={colorMode === "size"} onclick={() => (colorMode = "size")}>
          Size
        </button>
        <button type="button" class:active={colorMode === "ext"} onclick={() => (colorMode = "ext")}>
          Extension
        </button>
      </div>
      <button
        type="button"
        class="action"
        class:active={searchOpen}
        disabled={!selectedId}
        onclick={() => (searchOpen = !searchOpen)}
      >
        Search
      </button>
      {#if selectedId}
        <a class="action" href={exportUrl(selectedId, "csv", scopeId)} download>Export</a>
      {/if}
      {#if hasCompleted}
        <button type="button" class="action" disabled={gcBusy} onclick={() => void handleGc()}>
          {gcBusy ? "GC…" : "GC"}
        </button>
      {/if}
      <div class="status">
        {#if gcMessage}
          <span class="pill ok">{gcMessage}</span>
        {/if}
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

  <nav class="views" aria-label="Views">
    {#each VIEWS as entry (entry.id)}
      <button type="button" class:active={view === entry.id} onclick={() => (view = entry.id)}>
        {entry.label}
      </button>
    {/each}
  </nav>

  {#if searchOpen && selectedId}
    <section class="search-drawer">
      <div class="panel-head">
        <h2>Search</h2>
        <button type="button" class="link" onclick={() => (searchOpen = false)}>Close</button>
      </div>
      <SearchPanel scanId={selectedId} scope={scopeId} onselect={openNode} bind:q={searchQuery} />
    </section>
  {/if}

  {#if view === "docker"}
    <main class="solo">
      <section class="panel">
        <div class="panel-head">
          <h2>Docker</h2>
          <span class="muted">read-only</span>
        </div>
        <div class="view-host">
          <DockerPanel
            scanId={selectedId ?? undefined}
            onreveal={(nodeId) => (scopeId = nodeId)}
          />
        </div>
      </section>
    </main>
  {:else if view === "schedules"}
    <main class="solo">
      <section class="panel">
        <div class="panel-head">
          <h2>Schedules</h2>
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host">
          <SchedulesPanel />
        </div>
      </section>
    </main>
  {:else if view === "duplicates" && selectedId}
    <main class="solo">
      <section class="panel">
        <div class="panel-head">
          <h2>Duplicates</h2>
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host">
          <DuplicatesPanel scanId={selectedId} scope={scopeId} />
        </div>
      </section>
    </main>
  {:else if view === "diff" && selectedId}
    <main class="solo">
      <section class="panel">
        <div class="panel-head">
          <h2>Diff</h2>
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host">
          <DiffPanel scanId={selectedId} scans={scans} />
        </div>
      </section>
    </main>
  {:else if !selectedId}
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
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host" bind:this={treemapHost}>
          {#if view === "treemap"}
            {#if loading && tiles.length === 0}
              <p class="state">Loading tiles…</p>
            {:else if error}
              <p class="state err">{error}</p>
            {:else if tiles.length === 0}
              <p class="state">No tiles in {scopeLabel}.</p>
            {:else}
              <Treemap {tiles} {colorMode} onselect={openTile} />
            {/if}
          {:else if isHierarchyView}
            {#if viewLoading}
              <p class="state">Loading hierarchy…</p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if tree}
              {#if (tree.children ?? []).length > 0}
                {#if view === "sunburst"}
                  <Sunburst root={tree} onselect={openHierarchy} />
                {:else if view === "icicle"}
                  <Icicle root={tree} onselect={openHierarchy} />
                {:else}
                  <Bubble root={tree} onselect={openHierarchy} />
                {/if}
              {:else}
                <p class="state">No children in {scopeLabel}.</p>
              {/if}
            {:else}
              <p class="state">No children in {scopeLabel}.</p>
            {/if}
          {:else if view === "bars"}
            {#if viewLoading}
              <p class="state">Loading…</p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if topNodes.length === 0}
              <p class="state">No nodes in {scopeLabel}.</p>
            {:else}
              <Bars items={topNodes} onselect={openNode} />
            {/if}
          {:else if view === "heatmap"}
            {#if viewLoading}
              <p class="state">Loading…</p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if heatmapData}
              <Heatmap heatmap={heatmapData} />
            {:else}
              <p class="state">No data in {scopeLabel}.</p>
            {/if}
          {:else if view === "histogram"}
            {#if viewLoading}
              <p class="state">Loading…</p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if histogram.length === 0}
              <p class="state">No data in {scopeLabel}.</p>
            {:else}
              <Histogram items={histogram} />
            {/if}
          {:else}
            {#if viewLoading}
              <p class="state">Loading…</p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if histogram.length === 0}
              <p class="state">No data in {scopeLabel}.</p>
            {:else}
              <Breakdown items={histogram} title={breakdownTitle} />
            {/if}
          {/if}
        </div>
      </section>

      <section class="panel">
        <div class="panel-head">
          <h2>Ranked</h2>
          <span class="muted">{childTotal.toLocaleString()} items</span>
        </div>
        <RankedTable items={children} onselect={openNode} />
      </section>
    </main>
  {/if}

  <footer>
    <span>protocol v{health?.protocol ?? "—"}</span>
    <span class="muted">read-only · no telemetry</span>
  </footer>

  <ScanProgressPanel onfinished={handleScanFinished} />
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

  .action {
    padding: 0.3rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.75rem;
    text-decoration: none;
    cursor: pointer;
  }

  .action:hover {
    border-color: var(--accent);
  }

  .action.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .action:disabled {
    opacity: 0.5;
    cursor: default;
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

  .views {
    display: flex;
    gap: 0.25rem;
    padding: 0.4rem 1rem;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  .views button {
    padding: 0.3rem 0.7rem;
    border: 1px solid transparent;
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.75rem;
    white-space: nowrap;
    cursor: pointer;
  }

  .views button:hover {
    color: var(--text);
    background: var(--panel-2);
  }

  .views button.active {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--text);
  }

  .search-drawer {
    display: flex;
    flex-direction: column;
    max-height: 40vh;
    overflow: hidden;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  main {
    flex: 1;
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 1rem;
    padding: 1rem;
    min-height: 0;
  }

  main.solo {
    grid-template-columns: 1fr;
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

  .crumbs {
    min-width: 0;
    overflow: hidden;
  }

  .view-host {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
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
