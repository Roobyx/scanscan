<script lang="ts">
  import type {
    DriveInfo,
    HealthResponse,
    HeatmapResponse,
    HierarchyNode,
    HistogramBucket,
    NodeRecord,
    ScanSummary,
    Tile,
  } from "@scanscan/api-types";
  import { onDestroy, onMount } from "svelte";

  import Bars from "./components/Bars.svelte";
  import Breadcrumb from "./components/Breadcrumb.svelte";
  import Breakdown from "./components/Breakdown.svelte";
  import Bubble from "./components/Bubble.svelte";
  import DiffPanel from "./components/DiffPanel.svelte";
  import DockerPanel from "./components/DockerPanel.svelte";
  import DriveStats from "./components/DriveStats.svelte";
  import DrivesDashboard from "./components/DrivesDashboard.svelte";
  import DuplicatesPanel from "./components/DuplicatesPanel.svelte";
  import FileTypes from "./components/FileTypes.svelte";
  import Heatmap from "./components/Heatmap.svelte";
  import Histogram from "./components/Histogram.svelte";
  import Icicle from "./components/Icicle.svelte";
  import Icon from "./components/Icon.svelte";
  import Logo from "./components/Logo.svelte";
  import Modal from "./components/Modal.svelte";
  import RankedTable from "./components/RankedTable.svelte";
  import ScanLauncher from "./components/ScanLauncher.svelte";
  import ScanProgressPanel from "./components/ScanProgressPanel.svelte";
  import ScanSwitcher from "./components/ScanSwitcher.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SchedulesPanel from "./components/SchedulesPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import Spinner from "./components/Spinner.svelte";
  import Sunburst from "./components/Sunburst.svelte";
  import Treemap from "./components/Treemap.svelte";
  import {
    errorMessage,
    getChildren,
    getDrives,
    getHealth,
    getHeatmap,
    getHistogram,
    getScan,
    getTiles,
    getTop,
    getTree,
    listScans,
  } from "./lib/api.js";
  import type { HistogramDim } from "./lib/api.js";
  import type { ColorMode } from "./lib/color.js";
  import { fileTypeLabel, groupOfExtension } from "./lib/file-types.js";
  import { formatBytes } from "./lib/format.js";

  type ViewId =
    | "dashboard"
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
    | "schedules"
    | "settings";

  interface ViewDef {
    id: ViewId;
    label: string;
  }

  const VIEWS: ViewDef[] = [
    { id: "dashboard", label: "Drives" },
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
      view: viewParam === "settings" ? "settings" : matched ? matched.id : "dashboard",
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

  let drives = $state<DriveInfo[]>([]);
  let drivesHostRoot = $state("/host");
  let drivesWarning = $state<string | null>(null);
  let drivesError = $state<string | null>(null);
  let drivesLoading = $state(false);

  let scopePath = $state<Crumb[]>([]);
  let scopeId = $state(0);

  let tiles = $state<Tile[]>([]);
  let childTotal = $state(0);
  let truncated = $state(false);

  let colorMode = $state<ColorMode>(initial.color);
  let view = $state<ViewId>(initial.view);
  let loading = $state(false);
  let tileBusy = $state(false);
  let childBusy = $state(false);
  let swhitching = $state(true);
  let error = $state<string | null>(null);

  let tree = $state<HierarchyNode | null>(null);
  let topNodes = $state<NodeRecord[]>([]);
  let histogram = $state<HistogramBucket[]>([]);
  /** File-type group the Extensions breakdown is filtered to, or null. */
  let extGroup = $state<string | null>(null);
  let heatmapData = $state<HeatmapResponse | null>(null);
  let viewLoading = $state(false);
  let viewError = $state<string | null>(null);

  let searchOpen = $state(initial.q.length > 0);
  let searchQuery = $state(initial.q);
  let launcherOpen = $state(false);

  /** Node id whose overlay tree is shown in the Docker side panel, or null when closed. */
  let sideTreeNodeId = $state<number | null>(null);

  let treemapHost = $state<HTMLDivElement | null>(null);
  let hostW = $state(0);
  let hostH = $state(0);
  let tilesSeq = 0;
  let childrenSeq = 0;
  let viewSeq = 0;
  let scanSeq = 0;
  let switchTimer: ReturnType<typeof setTimeout> | undefined;

  onDestroy(() => clearTimeout(switchTimer));

  /** Veil the shell while another snapshot is selected and its data arrives. */
  function beginSwitch(): void {
    swhitching = true;
    clearTimeout(switchTimer);
    switchTimer = setTimeout(() => (swhitching = false), 8000);
  }

  function endSwitch(): void {
    clearTimeout(switchTimer);
    swhitching = false;
  }

  function bucket(value: number): number {
    if (value <= 0) return 0;
    return Math.max(256, Math.round(value / 128) * 128);
  }

  const tileW = $derived(bucket(hostW));
  const tileH = $derived(bucket(hostH));
  const scopeLabel = $derived(scopePath[scopePath.length - 1]?.name ?? "/");
  const totalBytes = $derived(tiles.reduce((sum, tile) => sum + tile.size, 0));
  const isHierarchyView = $derived(view === "sunburst" || view === "icicle" || view === "bubble");

  const viewSummary = $derived.by(() => {
    if (view === "dashboard") return `${scans.length} snapshots`;
    if (view === "treemap") return `${formatBytes(totalBytes)}${truncated ? " · truncated" : ""}`;
    if (isHierarchyView) return tree ? formatBytes(tree.size) : "—";
    if (view === "bars") return `${topNodes.length} nodes`;
    if (view === "docker") return "containers";
    if (view === "duplicates") return "duplicate files";
    if (view === "diff") return "snapshot diff";
    if (view === "schedules") return "cron jobs";
    return `${histogram.length} buckets`;
  });

  /** Extensions breakdown rows, narrowed to the chosen file-type group. */
  const extItems = $derived.by(() =>
    extGroup === null
      ? histogram
      : histogram.filter((bucket) => groupOfExtension(bucket.key) === extGroup),
  );

  const breakdownTitle = $derived.by(() => {
    if (view === "extensions") {
      return extGroup === null ? "Extensions" : `Extensions · ${fileTypeLabel(extGroup)}`;
    }
    return view === "age" ? "Age" : "Owners";
  });

  const scopePathIds = $derived.by(() => scopePath.map((crumb) => crumb.id));

  /** The filesystem the selected snapshot scann, matched to the host drives. */
  const activeDrive = $derived.by(() => {
    const summary = scans.find((scan) => scan.id === selectedId);
    const root = summary ? summary.roots[0] ?? "" : "";
    if (root.length === 0) return null;
    const same = drives.find((drive) => drive.containerPath === root);
    if (same) return same;
    let best: DriveInfo | null = null;
    for (const drive of drives) {
      if (root.slice(0, drive.containerPath.length) !== drive.containerPath) continue;
      if (!best || drive.containerPath.length > best.containerPath.length) best = drive;
    }
    return best;
  });

  /** The starting root of the snapshot, always the top row of the tree panel. */
  const treeRoot = $derived.by(() => {
    const root = scopePath[0];
    if (!root) return null;
    const summary = scans.find((scan) => scan.id === selectedId);
    return {
      id: root.id,
      name: root.name,
      size: summary?.bytesAlloc ?? 0,
      items: (summary?.files ?? 0) + (summary?.dirs ?? 0),
    };
  });

  async function loadScans(): Promise<void> {
    try {
      scans = await listScans();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function loadDrives(): Promise<void> {
    drivesLoading = true;
    drivesError = null;
    try {
      const response = await getDrives();
      drives = response.drives;
      drivesHostRoot = response.hostRoot;
      drivesWarning = response.warning ?? null;
    } catch (cause) {
      drivesError = errorMessage(cause);
    } finally {
      drivesLoading = false;
    }
  }

  async function selectScan(id: string): Promise<void> {
    const seq = scanSeq + 1;
    scanSeq = seq;
    const changed = id !== selectedId;
    selectedId = id;
    if (changed) beginSwitch();
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
    if (seq !== scanSeq) return;
    scopePath = [{ id: requested ?? 0, name: summary?.roots[0] ?? "/" }];
    extGroup = null;
    // The treemap drops the veiled shell once its tiles for the new snapshot
    // arrive; every other view has nothing left to wait for.
    if (view !== "treemap") endSwitch();
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

  async function handleScansChanged(): Promise<void> {
    await loadScans();
    if (!selectedId) return;
    if (scans.some((scan) => scan.id === selectedId)) return;
    const newest = scans
      .filter((scan) => scan.state === "completed")
      .sort((a, b) => (b.finishedAtMs ?? b.startedAtMs) - (a.finishedAtMs ?? a.startedAtMs))[0];
    if (newest) {
      await selectScan(newest.id);
    } else {
      selectedId = null;
      scopeId = 0;
      scopePath = [];
    }
  }

  function openLauncher(): void {
    launcherOpen = true;
  }

  async function handleLauncherCreated(id: string): Promise<void> {
    launcherOpen = false;
    await handleCreated(id);
  }

  /** Open a drive's snapshot from the dashboard. */
  async function openScan(id: string): Promise<void> {
    if (id.length === 0) return;
    view = "treemap";
    await selectScan(id);
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
      const response = await getTiles(scanId, { scope, depth: 1, w: width, h: height, color });
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
      if (seq !== tilesSeq) return;
      loading = false;
      endSwitch();
    }
  }

  async function loadChildren(scanId: string, scope: number): Promise<void> {
    const seq = childrenSeq + 1;
    childrenSeq = seq;
    childBusy = true;
    try {
      const response = await getChildren(scanId, scope, { sort: "size", limit: 1000 });
      if (seq !== childrenSeq) return;
      childTotal = response.total;
    } catch (cause) {
      if (seq !== childrenSeq) return;
      error = errorMessage(cause);
    } finally {
      if (seq === childrenSeq) childBusy = false;
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
    extGroup = null;
  }

  async function openTile(tile: Tile): Promise<void> {
    const scanId = selectedId;
    if (!scanId) return;
    tileBusy = true;
    try {
      const response = await getChildren(scanId, tile.node, { limit: 1, sort: "name" });
      if (response.total === 0) return;
      pushScope(tile.node, tile.name);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      tileBusy = false;
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
    tileBusy = true;
    try {
      const response = await getChildren(scanId, node.id, { limit: 1, sort: "name" });
      if (response.total === 0) return;
      pushScope(node.id, node.name);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      tileBusy = false;
    }
  }

  function navigate(index: number): void {
    const target = scopePath[index];
    if (!target) return;
    scopePath = scopePath.slice(0, index + 1);
    scopeId = target.id;
    extGroup = null;
  }

  async function openManagedScan(id: string): Promise<void> {
    view = "treemap";
    await selectScan(id);
  }

  onMount(() => {
    void (async () => {
      // Arm the fallback for the initial veil: a hung API must never leave the
      // shell covered with no way to reach the launcher.
      beginSwitch();
      try {
        health = await getHealth();
      } catch (cause) {
        healthError = errorMessage(cause);
      }
      try {
        void loadDrives();
        await loadScans();
        const completed = scans
          .filter((scan) => scan.state === "completed")
          .sort((a, b) => (b.finishedAtMs ?? b.startedAtMs) - (a.finishedAtMs ?? a.startedAtMs));
        const newest = completed[0];
        if (newest) await selectScan(newest.id);
        // Nothing scann yet: the dashboard is empty, so offer the launcher.
        if (!newest) launcherOpen = true;
        // The treemap drops the veil once its tiles arrive; every other case has
        // nothing left to wait for.
        if (view !== "treemap" || !newest) endSwitch();
      } catch (cause) {
        error = errorMessage(cause);
        endSwitch();
      }
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
      active === "dashboard" ||
      active === "treemap" ||
      active === "docker" ||
      active === "duplicates" ||
      active === "diff" ||
      active === "schedules" ||
      active === "settings"
    )
      return;
    void loadViewData();
  });

  $effect(() => {
    if (typeof window === "undefined") return;
    const params = new URLSearchParams();
    // The dashboard is the default view, so it stays out of the hash.
    if (view !== "dashboard") params.set("view", view);
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
      <Logo size={30} />
      <div class="brand-text">
        <h1>Scanscan</h1>
        <span class="tagline">disk analyzer</span>
      </div>
    </div>
    <div class="controls">
      <ScanSwitcher
        {scans}
        {selectedId}
        {drives}
        disabled={swhitching}
        onselect={(id) => void selectScan(id)}
      />
      <button type="button" class="action primary" onclick={openLauncher}>
        <Icon name="plus" size={14} />
        New scan
      </button>
      <div class="toggle" role="group" aria-label="Color mode">
        <button type="button" class:active={colorMode === "size"} onclick={() => (colorMode = "size")}>
          <Icon name="pie-chart" size={13} />
          Size
        </button>
        <button type="button" class:active={colorMode === "ext"} onclick={() => (colorMode = "ext")}>
          <Icon name="file" size={13} />
          Extension
        </button>
      </div>
      <button
        type="button"
        class="action"
        class:active={view === "settings"}
        aria-current={view === "settings" ? "page" : undefined}
        onclick={() => (view = "settings")}
      >
        <Icon name="settings" size={14} />
        Settings
      </button>
    </div>
  </header>

  <nav class="views" aria-label="Views">
    <div class="tabs">
      {#each VIEWS as entry (entry.id)}
        <button type="button" class:active={view === entry.id} onclick={() => (view = entry.id)}>
          {entry.label}
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="search-toggle"
      class:active={searchOpen}
      disabled={!selectedId}
      aria-pressed={searchOpen}
      onclick={() => (searchOpen = !searchOpen)}
    >
      <Icon name="search" size={14} />
      Search
    </button>
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

  {#if view === "dashboard"}
    <main class="dash">
      <section class="panel">
        <div class="panel-head">
          <h2>Drives</h2>
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host">
          <DrivesDashboard
            scans={scans}
            drives={drives}
            hostRoot={drivesHostRoot}
            warning={drivesWarning}
            error={drivesError}
            loading={drivesLoading}
            onrefresh={loadDrives}
            onopen={openScan}
            oncreated={handleCreated}
          />
        </div>
      </section>
    </main>
  {:else if view === "settings"}
    <main class="settings-main">
      <SettingsPanel
        {scans}
        {selectedId}
        {drives}
        {health}
        {healthError}
        onopen={openManagedScan}
        onchanged={handleScansChanged}
      />
    </main>
  {:else if view === "docker"}
    <main class="solo">
      <section class="panel">
        <div class="panel-head">
          <h2>Docker</h2>
          <span class="muted">read-only</span>
        </div>
        <div class="view-host">
          <DockerPanel
            scanId={selectedId ?? undefined}
            onreveal={(nodeId) => { scopeId = nodeId; }}
          />
        </div>
        {#if sideTreeNodeId !== null}
          <section class="side-tree-panel">
            <div class="panel-head">
              <h2>Container overlay tree</h2>
              <button type="button" class="close-btn" onclick={() => (sideTreeNodeId = null)} aria-label="Close tree panel">✕</button>
            </div>
            <div class="view-host">
              {#if sideTreeNodeId !== null && tree && (tree.children ?? []).length > 0}
                <Icicle root={tree} onselect={() => (sideTreeNodeId = null)} />
              {:else if sideTreeNodeId !== null}
                <p class="state">No tree data available for this node.</p>
              {/if}
            </div>
          </section>
        {/if}
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
      <section class="empty-card">
        <Logo size={46} />
        <h2>No snapshot selected</h2>
        <p>Start a read-only scan to explore disk usage on this host.</p>
        <button type="button" class="action primary" onclick={openLauncher}>
          <Icon name="plus" size={14} />
          New scan
        </button>
      </section>
    </main>
  {:else}
    <main>
      <section class="panel">
        <div class="panel-head">
          <div class="crumbs">
            <Breadcrumb path={scopePath} onnavigate={navigate} />
          </div>
          <DriveStats drive={activeDrive} />
          {#if loading || tileBusy}
            <span class="busy" role="status" aria-label="Loading view">
              <Spinner size={12} label="Loading view" />
              <span>Loading…</span>
            </span>
          {/if}
          <span class="muted">{viewSummary}</span>
        </div>
        <div class="view-host" class:stacked={view === "extensions"} bind:this={treemapHost}>
          {#if view === "treemap"}
            {#if loading && tiles.length === 0}
              <p class="state" role="status" aria-label="Loading tiles">
                <Spinner size={16} label="Loading tiles" />
                <span>Loading tiles…</span>
              </p>
            {:else if error}
              <p class="state err">{error}</p>
            {:else if tiles.length === 0}
              <p class="state">No tiles in {scopeLabel}.</p>
            {:else}
              <Treemap {tiles} {colorMode} onselect={openTile} />
            {/if}
          {:else if isHierarchyView}
            {#if viewLoading}
              <p class="state" role="status" aria-label="Loading hierarchy">
                <Spinner size={16} label="Loading hierarchy" />
                <span>Loading hierarchy…</span>
              </p>
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
              <p class="state" role="status" aria-label="Loading nodes">
                <Spinner size={16} label="Loading nodes" />
                <span>Loading nodes…</span>
              </p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if topNodes.length === 0}
              <p class="state">No nodes in {scopeLabel}.</p>
            {:else}
              <Bars items={topNodes} onselect={openNode} />
            {/if}
          {:else if view === "heatmap"}
            {#if viewLoading}
              <p class="state" role="status" aria-label="Loading heatmap">
                <Spinner size={16} label="Loading heatmap" />
                <span>Loading heatmap…</span>
              </p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if heatmapData}
              <Heatmap heatmap={heatmapData} />
            {:else}
              <p class="state">No data in {scopeLabel}.</p>
            {/if}
          {:else if view === "histogram"}
            {#if viewLoading}
              <p class="state" role="status" aria-label="Loading histogram">
                <Spinner size={16} label="Loading histogram" />
                <span>Loading histogram…</span>
              </p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if histogram.length === 0}
              <p class="state">No data in {scopeLabel}.</p>
            {:else}
              <Histogram items={histogram} />
            {/if}
          {:else if view === "extensions"}
            {#if viewLoading}
              <p class="state" role="status" aria-label="Loading breakdown">
                <Spinner size={16} label="Loading breakdown" />
                <span>Loading breakdown…</span>
              </p>
            {:else if viewError}
              <p class="state err">{viewError}</p>
            {:else if histogram.length === 0}
              <p class="state">No data in {scopeLabel}.</p>
            {:else}
              <FileTypes
                items={histogram}
                selected={extGroup}
                onselect={(group) => (extGroup = group)}
              />
              {#if extItems.length === 0}
                <p class="state">No files of this type in {scopeLabel}.</p>
              {:else}
                <Breakdown items={extItems} title={breakdownTitle} />
              {/if}
            {/if}
          {:else}
            {#if viewLoading}
              <p class="state" role="status" aria-label="Loading breakdown">
                <Spinner size={16} label="Loading breakdown" />
                <span>Loading breakdown…</span>
              </p>
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
          <h2>Tree</h2>
          <span class="muted">{childTotal.toLocaleString()} items in scope</span>
          {#if childBusy}
            <Spinner size={12} label="Loading items in scope" />
          {/if}
        </div>
        <RankedTable
          scanId={selectedId}
          root={treeRoot}
          path={scopePathIds}
          scope={scopeId}
          onselect={openNode}
        />
      </section>
    </main>
  {/if}

  <footer>
    <span>protocol v{health?.protocol ?? "—"}</span>
    <span class="muted">read-only · no telemetry</span>
  </footer>

  {#if swhitching}
    <div class="overlay" role="status" aria-label="Loading snapshot">
      <div class="overlay-card">
        <Spinner size={26} label="Loading snapshot" />
        <span>Loading snapshot…</span>
      </div>
    </div>
  {/if}

  {#if launcherOpen}
    <Modal
      title="New scan"
      subtitle="Read-only scan of a host filesystem"
      onclose={() => (launcherOpen = false)}
    >
      <ScanLauncher oncreated={handleLauncherCreated} />
    </Modal>
  {/if}

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
    padding: 0.65rem 1rem;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--panel) 82%, var(--accent) 18%),
      var(--panel)
    );
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }

  h1 {
    margin: 0;
    font-size: 1.05rem;
    letter-spacing: 0.01em;
  }

  .tagline {
    color: var(--muted);
    font-size: 0.62rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    min-width: 0;
  }

  .toggle {
    display: inline-flex;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel-2);
  }

  .toggle button {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.35rem 0.6rem;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.73rem;
    cursor: pointer;
  }

  .toggle button:hover {
    color: var(--text);
  }

  .toggle button.active {
    background: var(--accent);
    color: #0b0e14;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 0.32rem;
    padding: 0.35rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.74rem;
    text-decoration: none;
    cursor: pointer;
  }

  .action:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .action.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .action.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #0b0e14;
    font-weight: 600;
  }

  .action.primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .muted {
    color: var(--muted);
  }

  .views {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }

  .tabs {
    display: flex;
    flex: 1;
    gap: 0.25rem;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .tabs button,
  .search-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
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

  .tabs button:hover,
  .search-toggle:hover:not(:disabled) {
    color: var(--text);
    background: var(--panel-2);
  }

  .tabs button.active,
  .search-toggle.active {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--text);
  }

  .search-toggle {
    flex: none;
    border-color: var(--border);
  }

  .search-toggle:disabled {
    opacity: 0.45;
    cursor: default;
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

  main.dash {
    display: block;
    overflow: auto;
  }

  main.dash .panel {
    margin-bottom: 1rem;
  }

  main.empty-main {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  main.settings-main {
    display: flex;
    flex: 1;
    min-height: 0;
    padding: 0;
    overflow: hidden;
  }

  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.6rem;
    width: min(28rem, 90vw);
    padding: 2rem 1.5rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--panel);
    text-align: center;
  }

  .empty-card h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .empty-card p {
    margin: 0 0 0.4rem;
    color: var(--muted);
    font-size: 0.82rem;
    line-height: 1.5;
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

  .view-host.stacked {
    flex-direction: column;
  }

  .state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    margin: auto;
    padding: 1rem;
    color: var(--muted);
  }

  .state.err {
    color: var(--err);
  }

  .busy {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
    font-size: 0.75rem;
    white-space: nowrap;
  }

  .overlay {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background: color-mix(in srgb, var(--bg) 62%, transparent);
  }

  .overlay-card {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.9rem 1.2rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
    color: var(--text);
    font-size: 0.85rem;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
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
