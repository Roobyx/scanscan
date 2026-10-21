<script lang="ts">
  import type {
    ContainerInfo,
    DockerStats,
    ImageInfo,
    LocateMatch,
    VolumeInfo,
  } from "@scanscan/api-types";
  import { onMount } from "svelte";

  import {
    errorMessage,
    getDockerContainers,
    getDockerImages,
    getDockerLocate,
    getDockerOverlayContents,
    getDockerStats,
    getDockerVolumes,
  } from "../lib/api.js";
  import { formatBytes, formatAgo, percent } from "../lib/format.js";

  import Spinner from "./Spinner.svelte";

  interface Props {
    scanId?: string;
    onreveal?: (nodeId: number) => void;
  }

  let { scanId, onreveal }: Props = $props();

  let containers = $state<ContainerInfo[]>([]);
  let images = $state<ImageInfo[]>([]);
  let volumes = $state<VolumeInfo[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;
  let statsById = $state<Map<string, DockerStats>>(new Map());
  let locateQuery = $state("");
  let locateMatches = $state<LocateMatch[]>([]);
  let locating = $state(false);

  type ContainerView = "cards" | "table";
  type ContainerSort = "size" | "created" | "state";

  let view: ContainerView = $state("cards");
  let sortBy: ContainerSort = $state("size");
  let sortDir: "asc" | "desc" = $state("desc");

  const sortedContainers = $derived(
    [...containers].sort((a, b) => {
      let cmp = 0;
      if (sortBy === "size") cmp = (a.sizeRw ?? 0) - (b.sizeRw ?? 0);
      else if (sortBy === "created") cmp = (a.created ?? 0) - (b.created ?? 0);
      else cmp = a.state.localeCompare(b.state);
      return sortDir === "desc" ? -cmp : cmp;
    }),
  );

  async function locate(): Promise<void> {
    const query = locateQuery.trim();
    if (query.length === 0) {
      locateMatches = [];
      return;
    }
    locating = true;
    try {
      locateMatches = await getDockerLocate(query);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      locating = false;
    }
  }

  const sortedImages = $derived([...images].sort((a, b) => b.size - a.size));
  const sortedVolumes = $derived(
    [...volumes].sort((a, b) => (b.size ?? -1) - (a.size ?? -1)),
  );
  const isEmpty = $derived(
    containers.length === 0 && images.length === 0 && volumes.length === 0,
  );

  // The containers response already carries each container's mounts; the
  // flattened view for the Mounts table is derived client-side so the panel
  // does not fetch the same data twice.
  const allMounts = $derived(containers.flatMap((container) => container.mounts));

  function mountKey(mount: ContainerInfo["mounts"][number]): string {
    return `${mount.containerId}:${mount.destination}:${mount.source}`;
  }

  function repoLabel(image: ImageInfo): string {
    return image.repoTags.length > 0 ? image.repoTags.join(", ") : "<none>";
  }

  function reveal(mount: ContainerInfo["mounts"][number]): void {
    if (mount.node !== undefined) onreveal?.(mount.node);
  }

  async function exploreOverlay(
    scanId: string,
    containerId: string,
    layer: "upper" | "lower" | "merged" | "",
  ): Promise<void> {
    try {
      const result = await getDockerOverlayContents(scanId, containerId, layer);
      if (result.nodeId !== null) {
        onreveal?.(result.nodeId);
      }
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function loadInitial(id: string | undefined): Promise<void> {
    loading = true;
    error = null;
    try {
      const [nextContainers, nextImages, nextVolumes] = await Promise.all([
        getDockerContainers(id),
        getDockerImages(),
        getDockerVolumes(),
      ]);
      containers = nextContainers;
      images = nextImages;
      volumes = nextVolumes;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      loading = false;
    }
  }

  async function pollStats(): Promise<void> {
    try {
      statsById = indexStats(await getDockerStats());
    } catch {
      // Keep the last sample; a transient docker hiccup should not clear the view.
    }
  }

  function indexStats(samples: DockerStats[]): Map<string, DockerStats> {
    return new Map(samples.map((sample) => [sample.id, sample]));
  }

  onMount(() => {
    timer = setInterval(() => {
      void pollStats();
    }, 2000);
    return () => {
      if (timer !== null) clearInterval(timer);
    };
  });

  $effect(() => {
    void loadInitial(scanId);
  });
</script>

<div class="docker">
  <section class="block">
    <h3>Locate a path or id</h3>
    <p class="muted small">
      Paste a host path or an id fragment (e.g. an overlay layer id) to find the container/stack
      that owns it.
    </p>
    <form
      class="locate"
      onsubmit={(event) => {
        event.preventDefault();
        void locate();
      }}
    >
      <input
        type="text"
        bind:value={locateQuery}
        placeholder="ff50410b4133… or /mnt/…"
        aria-label="Path or id"
      />
      <button type="submit" disabled={locating}>{locating ? "…" : "Locate"}</button>
    </form>
    {#if locateMatches.length > 0}
      <ul class="matches">
        {#each locateMatches as match (match.containerId + match.kind + match.path)}
          <li>
            <strong>{match.containerName}</strong>
            <span class="muted small">{match.kind}</span>
            <span class="mono" title={match.path}>{match.path}</span>
          </li>
        {/each}
      </ul>
    {:else if locateQuery.trim().length > 0 && !locating}
      <p class="muted small">No container matches “{locateQuery}”.</p>
    {/if}
  </section>

  {#if loading && !isEmpty}
    <p class="busy" role="status" aria-label="Refreshing Docker">
      <Spinner size={12} label="Refreshing Docker" />
      <span>Refreshing Docker…</span>
    </p>
  {/if}

  {#if error}
    <p class="state err" role="alert">{error}</p>
  {:else if loading && isEmpty}
    <p class="state" role="status" aria-label="Loading Docker">
      <Spinner size={16} label="Loading Docker" />
      <span>Loading Docker…</span>
    </p>
  {:else}
    <section class="block">
      <div class="container-controls">
        <h3>Containers <span class="muted">{containers.length}</span></h3>
        <div class="container-toolbar">
          <span class="toolbar-label" id="view-label">View</span>
          <div class="toggle-group" role="group" aria-labelledby="view-label">
            <button
              type="button"
              class:active={view === "cards"}
              onclick={() => (view = "cards")}
              aria-pressed={view === "cards"}
            >
              Cards
            </button>
            <button
              type="button"
              class:active={view === "table"}
              onclick={() => (view = "table")}
              aria-pressed={view === "table"}
            >
              Table
            </button>
          </div>
          <label class="toolbar-label" for="container-sort">Sort by</label>
          <select id="container-sort" bind:value={sortBy}>
            <option value="size">Size</option>
            <option value="created">Date created</option>
            <option value="state">Running state</option>
          </select>
          <button
            type="button"
            class:desc={sortDir === "desc"}
            onclick={() => (sortDir = sortDir === "desc" ? "asc" : "desc")}
            aria-label="Toggle sort direction"
          >
            {sortDir === "desc" ? "▾" : "▴"}
          </button>
        </div>
      </div>
      {#if containers.length === 0}
        <p class="muted small">No containers found. Docker may be unavailable.</p>
      {:else if view === "cards"}
        <div class="cards">
          {#each sortedContainers as container (container.id)}
            {@const sample = statsById.get(container.id)}
            <article class="card">
              <header>
                <span class="name" title={container.displayName ?? container.name}>{container.displayName ?? container.name}</span>
                <span class="state-pill" class:running={container.state === "running"}>
                  {container.state}
                </span>
              </header>
              <p class="image" title={container.image}>{container.image}</p>
              {#if container.composeProject}
                <p class="image compose">
                  {container.composeProject}{container.composeService
                    ? `/${container.composeService}`
                    : ""}
                </p>
              {/if}
              <dl>
                <div><dt>RW</dt><dd>{formatBytes(container.sizeRw ?? 0)}</dd></div>
                <div><dt>RootFS</dt><dd>{formatBytes(container.sizeRootFs ?? 0)}</dd></div>
                <div><dt>Mounts</dt><dd>{container.mounts.length}</dd></div>
                {#if container.created !== undefined}
                  <div><dt>Created</dt><dd>{formatAgo(container.created / 1e6)}</dd></div>
                {/if}
              </dl>
              {#if sample}
                <div class="card-meters">
                  <span class="meter">
                    <span class="meter-label">CPU {sample.cpuPercent.toFixed(1)}%</span>
                    <span class="meter-track">
                      <span class="meter-fill cpu" style="width: {percent(sample.cpuPercent, 100)}%"></span>
                    </span>
                  </span>
                  <span class="meter">
                    <span class="meter-label">MEM {formatBytes(sample.memUsed)} / {formatBytes(sample.memLimit)}</span>
                    <span class="meter-track">
                      <span class="meter-fill mem" style="width: {percent(sample.memUsed, sample.memLimit)}%"></span>
                    </span>
                  </span>
                </div>
              {/if}
              {#if scanId && (container.overlayUpper || container.overlayLower || container.overlayMerged)}
                <p class="overlay-links">
                  {#if container.overlayUpper}
                    <button
                      type="button"
                      class="node-link"
                      title="Explore overlay upper layer"
                      onclick={() => void exploreOverlay(scanId, container.id, "upper")}
                    >
                      Upper
                    </button>
                  {/if}
                  {#if container.overlayLower}
                    <button
                      type="button"
                      class="node-link"
                      title="Explore overlay lower layers"
                      onclick={() => void exploreOverlay(scanId, container.id, "lower")}
                    >
                      Lower
                    </button>
                  {/if}
                  {#if container.overlayMerged}
                    <button
                      type="button"
                      class="node-link"
                      title="Explore merged overlay"
                      onclick={() => void exploreOverlay(scanId, container.id, "merged")}
                    >
                      Merged
                    </button>
                  {/if}
                </p>
              {/if}
            </article>
          {/each}
        </div>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>State</th>
                <th class="ellipsis" title="Image">Image</th>
                <th class="num">Size RW</th>
                <th class="num">Created</th>
                <th class="num">Mounts</th>
                <th class="num">CPU</th>
                <th class="num">MEM</th>
                {#if scanId}
                  <th class="num">Overlay</th>
                {/if}
              </tr>
            </thead>
            <tbody>
              {#each sortedContainers as container (container.id)}
                {@const sample = statsById.get(container.id)}
                <tr>
                  <td class="ellipsis" title={container.displayName ?? container.name}>
                    {container.displayName ?? container.name}
                  </td>
                  <td>
                    <span class="state-pill" class:running={container.state === "running"}>
                      {container.state}
                    </span>
                  </td>
                  <td class="ellipsis" title={container.image}>{container.image}</td>
                  <td class="num">{formatBytes(container.sizeRw ?? 0)}</td>
                  <td class="num">
                    {#if container.created !== undefined}
                      {formatAgo(container.created / 1e6)}
                    {:else}
                      —
                    {/if}
                  </td>
                  <td class="num">{container.mounts.length}</td>
                  <td class="num">{sample ? `${sample.cpuPercent.toFixed(1)}%` : "—"}</td>
                  <td class="num">
                    {#if sample}
                      {formatBytes(sample.memUsed)}
                    {:else}
                      —
                    {/if}
                  </td>
                  {#if scanId}
                    <td class="num">
                      {#if container.overlayUpper}
                        <button
                          type="button"
                          class="node-link"
                          title="Explore overlay upper layer"
                          onclick={() => void exploreOverlay(scanId, container.id, "upper")}
                        >
                          Upper
                        </button>
                      {:else}
                        —
                      {/if}
                      {#if container.overlayLower}
                        <button
                          type="button"
                          class="node-link"
                          title="Explore overlay lower layers"
                          onclick={() => void exploreOverlay(scanId, container.id, "lower")}
                        >
                          Lower
                        </button>
                      {/if}
                      {#if container.overlayMerged}
                        <button
                          type="button"
                          class="node-link"
                          title="Explore merged overlay"
                          onclick={() => void exploreOverlay(scanId, container.id, "merged")}
                        >
                          Merged
                        </button>
                      {/if}
                      {#if !container.overlayUpper && !container.overlayLower && !container.overlayMerged}
                        —
                      {/if}
                    </td>
                  {/if}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="block">
      <h3>Images <span class="muted">{images.length}</span></h3>
      {#if images.length === 0}
        <p class="muted small">No images reported.</p>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Repository</th>
                <th class="num">Size</th>
                <th class="num">Shared</th>
                <th class="num">Containers</th>
              </tr>
            </thead>
            <tbody>
              {#each sortedImages as image (image.id)}
                <tr>
                  <td class="ellipsis" title={repoLabel(image)}>{repoLabel(image)}</td>
                  <td class="num">{formatBytes(image.size)}</td>
                  <td class="num">{formatBytes(image.sharedSize)}</td>
                  <td class="num">{image.containers}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="block">
      <h3>Volumes <span class="muted">{volumes.length}</span></h3>
      {#if volumes.length === 0}
        <p class="muted small">No volumes reported.</p>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Driver</th>
                <th class="num">Size</th>
                <th class="num">Refs</th>
              </tr>
            </thead>
            <tbody>
              {#each sortedVolumes as volume (volume.name)}
                <tr>
                  <td class="ellipsis" title={volume.name}>{volume.name}</td>
                  <td>{volume.driver}</td>
                  <td class="num">{volume.size === undefined ? "—" : formatBytes(volume.size)}</td>
                  <td class="num">{volume.refCount === undefined ? "—" : volume.refCount}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="block">
      <h3>Mounts <span class="muted">{allMounts.length}</span></h3>
      {#if allMounts.length === 0}
        <p class="muted small">No mounts reported.</p>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Container</th>
                <th>Kind</th>
                <th>Source → Destination</th>
                <th>Mode</th>
                <th class="num">Node</th>
                <th class="num">Size</th>
              </tr>
            </thead>
            <tbody>
              {#each allMounts as mount (mountKey(mount))}
                <tr>
                  <td class="ellipsis" title={mount.containerName}>{mount.containerName}</td>
                  <td>{mount.kind}</td>
                  <td class="ellipsis" title="{mount.source} → {mount.destination}">
                    <span class="src">{mount.source}</span>
                    <span class="arrow" aria-hidden="true">→</span>
                    <span class="dst">{mount.destination}</span>
                  </td>
                  <td>{mount.readWrite ? "rw" : "ro"}</td>
                  <td class="num">
                    {#if mount.node !== undefined}
                      <button
                        type="button"
                        class="node-link"
                        title="Reveal node #{mount.node}"
                        onclick={() => reveal(mount)}
                      >
                        {mount.node}
                      </button>
                    {:else}
                      —
                    {/if}
                  </td>
                  <td class="num">{mount.size === undefined ? "—" : formatBytes(mount.size)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .docker {
    flex: 1;
    min-height: 0;
    padding: 0.75rem;
    overflow: auto;
  }

  .block {
    margin-bottom: 1.25rem;
  }

  h3 {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    margin: 0 0 0.6rem;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .muted {
    color: var(--muted);
  }

  .small {
    font-size: 0.78rem;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.6rem;
  }

  .card {
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
  }

  .card header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .state-pill {
    padding: 0.1rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.7rem;
    white-space: nowrap;
  }

  .state-pill.running {
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 40%, transparent);
  }

  .image {
    margin: 0.35rem 0 0.5rem;
    overflow: hidden;
    color: var(--muted);
    font-size: 0.75rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .image.compose {
    margin-top: -0.35rem;
    color: var(--accent);
  }

  dl {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.3rem;
    margin: 0;
  }

  dl div {
    display: flex;
    flex-direction: column;
  }

  dt {
    color: var(--muted);
    font-size: 0.65rem;
    letter-spacing: 0.04em;
  }

  dd {
    margin: 0;
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }

  .container-controls {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.4rem;
    flex-wrap: wrap;
  }

  .container-controls h3 {
    margin: 0;
  }

  .container-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .toolbar-label {
    color: var(--muted);
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .toggle-group {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
  }

  .toggle-group button {
    padding: 0.2rem 0.55rem;
    border: none;
    border-right: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 600;
    cursor: pointer;
  }

  .toggle-group button:last-child {
    border-right: none;
  }

  .toggle-group button.active {
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: var(--accent);
  }

  .container-toolbar select {
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .container-toolbar button {
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .container-toolbar button.desc {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: var(--accent);
  }

  .card-meters {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-top: 0.5rem;
  }

  .overlay-links {
    display: flex;
    gap: 0.35rem;
    margin: 0.5rem 0 0;
  }

  .meter {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .meter-label {
    color: var(--muted);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
  }

  .meter-track {
    height: 8px;
    overflow: hidden;
    border-radius: 4px;
    background: color-mix(in srgb, var(--border) 60%, transparent);
  }

  .meter-fill {
    display: block;
    height: 100%;
    min-width: 2px;
  }

  .meter-fill.cpu {
    background: var(--warn);
  }

  .meter-fill.mem {
    background: var(--accent);
  }

  .table-wrap {
    overflow: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
  }

  th {
    padding: 0.4rem 0.5rem;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    max-width: 18rem;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  .ellipsis {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  th.num,
  td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .src,
  .dst {
    color: var(--text);
  }

  .arrow {
    margin: 0 0.35rem;
    color: var(--muted);
  }

  .node-link {
    padding: 0.05rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    color: var(--accent);
    font: inherit;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }

  .node-link:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    margin: 2rem auto;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }

  .busy {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 0.75rem;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .locate {
    display: flex;
    gap: 0.5rem;
    margin: 0.5rem 0;
  }

  .locate input {
    flex: 1;
    min-width: 0;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  .locate button {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--accent);
    color: #0b0e14;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .matches {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .matches li {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
    font-size: 0.8rem;
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--muted);
    font-size: 0.72rem;
    word-break: break-all;
  }
</style>
