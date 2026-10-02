<script lang="ts">
  import type {
    ContainerInfo,
    DockerStats,
    ImageInfo,
    MountInfo,
    VolumeInfo,
  } from "@scanscan/api-types";
  import { onMount } from "svelte";

  import {
    errorMessage,
    getDockerContainers,
    getDockerImages,
    getDockerMounts,
    getDockerStats,
    getDockerVolumes,
  } from "../lib/api.js";
  import { formatBytes, percent } from "../lib/format.js";

  interface Props {
    scanId?: string;
    onreveal?: (nodeId: number) => void;
  }

  let { scanId, onreveal }: Props = $props();

  let containers = $state<ContainerInfo[]>([]);
  let images = $state<ImageInfo[]>([]);
  let volumes = $state<VolumeInfo[]>([]);
  let mounts = $state<MountInfo[]>([]);
  let stats = $state<DockerStats[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  const sortedImages = $derived([...images].sort((a, b) => b.size - a.size));
  const sortedVolumes = $derived(
    [...volumes].sort((a, b) => (b.size ?? -1) - (a.size ?? -1)),
  );
  const isEmpty = $derived(
    containers.length === 0 && images.length === 0 && volumes.length === 0 && mounts.length === 0,
  );

  function mountKey(mount: MountInfo): string {
    return `${mount.containerId}:${mount.destination}:${mount.source}`;
  }

  function repoLabel(image: ImageInfo): string {
    return image.repoTags.length > 0 ? image.repoTags.join(", ") : "<none>";
  }

  function reveal(mount: MountInfo): void {
    if (mount.node !== undefined) onreveal?.(mount.node);
  }

  async function loadInitial(id: string | undefined): Promise<void> {
    loading = true;
    error = null;
    try {
      const [nextContainers, nextImages, nextVolumes, nextMounts, nextStats] = await Promise.all([
        getDockerContainers(),
        getDockerImages(),
        getDockerVolumes(),
        getDockerMounts(id),
        getDockerStats(),
      ]);
      containers = nextContainers;
      images = nextImages;
      volumes = nextVolumes;
      mounts = nextMounts;
      stats = nextStats;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      loading = false;
    }
  }

  async function pollStats(): Promise<void> {
    try {
      stats = await getDockerStats();
    } catch {
      // Keep the last sample; a transient docker hiccup should not clear the view.
    }
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
  {#if error}
    <p class="state err" role="alert">{error}</p>
  {:else if loading && isEmpty}
    <p class="state">Loading Docker…</p>
  {:else}
    <section class="block">
      <h3>Containers <span class="muted">{containers.length}</span></h3>
      {#if containers.length === 0}
        <p class="muted small">No containers found. Docker may be unavailable.</p>
      {:else}
        <div class="cards">
          {#each containers as container (container.id)}
            <article class="card">
              <header>
                <span class="name" title={container.name}>{container.name}</span>
                <span class="state-pill" class:running={container.state === "running"}>
                  {container.state}
                </span>
              </header>
              <p class="image" title={container.image}>{container.image}</p>
              <dl>
                <div><dt>RW</dt><dd>{formatBytes(container.sizeRw ?? 0)}</dd></div>
                <div><dt>RootFS</dt><dd>{formatBytes(container.sizeRootFs ?? 0)}</dd></div>
                <div><dt>Mounts</dt><dd>{container.mounts.length}</dd></div>
              </dl>
            </article>
          {/each}
        </div>
      {/if}
    </section>

    <section class="block">
      <h3>Stats <span class="muted">live · 2s</span></h3>
      {#if stats.length === 0}
        <p class="muted small">No running containers report stats.</p>
      {:else}
        <ul class="stats">
          {#each stats as sample (sample.id)}
            <li>
              <span class="stat-name" title={sample.name}>{sample.name}</span>
              <span class="meter">
                <span class="meter-label">CPU {sample.cpuPercent.toFixed(1)}%</span>
                <span class="meter-track">
                  <span
                    class="meter-fill cpu"
                    style="width: {percent(sample.cpuPercent, 100)}%"
                  ></span>
                </span>
              </span>
              <span class="meter">
                <span class="meter-label">
                  MEM {formatBytes(sample.memUsed)} / {formatBytes(sample.memLimit)}
                </span>
                <span class="meter-track">
                  <span
                    class="meter-fill mem"
                    style="width: {percent(sample.memUsed, sample.memLimit)}%"
                  ></span>
                </span>
              </span>
            </li>
          {/each}
        </ul>
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
      <h3>Mounts <span class="muted">{mounts.length}</span></h3>
      {#if mounts.length === 0}
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
              {#each mounts as mount (mountKey(mount))}
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

  .stats {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .stats li {
    display: grid;
    grid-template-columns: minmax(6rem, 12rem) 1fr 1fr;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.78rem;
  }

  .stat-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    margin: 2rem auto;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }
</style>
