<script lang="ts">
  import { onMount } from "svelte";

  import { formatBytes, percent } from "./lib/format.js";

  type Health = {
    status: string;
    protocol: number;
    core: { connected: boolean; socket: string };
  };

  type Region = { name: string; size: number; color: string };

  let health = $state<Health | null>(null);
  let error = $state<string | null>(null);

  // Placeholder dataset until the core serves real tiles (Phase 1).
  const regions: Region[] = [
    { name: "/var/lib/docker", size: 412 * 1024 ** 3, color: "#4f8cff" },
    { name: "/home", size: 268 * 1024 ** 3, color: "#3ddc97" },
    { name: "/usr", size: 96 * 1024 ** 3, color: "#ffb454" },
    { name: "/var/log", size: 22 * 1024 ** 3, color: "#b98cff" },
    { name: "/opt", size: 14 * 1024 ** 3, color: "#ff6b6b" },
    { name: "/etc", size: 3 * 1024 ** 3, color: "#5ad1e6" },
  ];

  const total = $derived(regions.reduce((sum, r) => sum + r.size, 0));

  onMount(async () => {
    try {
      const res = await fetch("/api/v1/health");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      health = (await res.json()) as Health;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });
</script>

<div class="shell">
  <header>
    <div class="brand">
      <span class="logo">▚</span>
      <h1>scanscan</h1>
    </div>
    <div class="status">
      {#if error}
        <span class="pill err">API unreachable</span>
      {:else if health}
        <span class="pill ok">API {health.status}</span>
        <span class="pill {health.core.connected ? 'ok' : 'muted'}">
          core {health.core.connected ? "connected" : "idle"}
        </span>
      {:else}
        <span class="pill muted">connecting…</span>
      {/if}
    </div>
  </header>

  <main>
    <section class="panel">
      <div class="panel-head">
        <h2>Disk usage</h2>
        <span class="muted">{formatBytes(total)} total · placeholder data</span>
      </div>
      <div class="treemap">
        {#each regions as region (region.name)}
          <div
            class="cell"
            style="flex-grow: {region.size}; background: {region.color}"
            title="{region.name} — {formatBytes(region.size)}"
          >
            <span class="cell-label">{region.name}</span>
            <span class="cell-size">{formatBytes(region.size)}</span>
            <span class="cell-pct">{percent(region.size, total).toFixed(1)}%</span>
          </div>
        {/each}
      </div>
    </section>

    <section class="panel">
      <div class="panel-head">
        <h2>Largest directories</h2>
        <span class="muted">ranked by allocated size</span>
      </div>
      <ul class="ranked">
        {#each [...regions].sort((a, b) => b.size - a.size) as region (region.name)}
          <li>
            <span class="swatch" style="background: {region.color}"></span>
            <span class="rank-name">{region.name}</span>
            <span class="rank-size">{formatBytes(region.size)}</span>
          </li>
        {/each}
      </ul>
    </section>
  </main>

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

  .status {
    display: flex;
    gap: 0.5rem;
  }

  .pill {
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.75rem;
    border: 1px solid var(--border);
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

  .panel {
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    min-height: 0;
  }

  .panel-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
  }

  .panel-head h2 {
    margin: 0;
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .treemap {
    flex: 1;
    display: flex;
    gap: 2px;
    padding: 2px;
    min-height: 0;
  }

  .cell {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 0.1rem;
    padding: 0.5rem;
    min-width: 0;
    overflow: hidden;
    color: #0b0e14;
    font-weight: 600;
  }

  .cell-label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cell-size,
  .cell-pct {
    font-size: 0.7rem;
    opacity: 0.75;
  }

  .ranked {
    list-style: none;
    margin: 0;
    padding: 0.5rem;
    overflow: auto;
  }

  .ranked li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
    border-radius: 6px;
  }

  .ranked li:hover {
    background: var(--panel-2);
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }

  .rank-size {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
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
