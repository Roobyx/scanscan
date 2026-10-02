<script lang="ts">
  import { onMount } from "svelte";

  import type { HostMount } from "@scanscan/api-types";
  import { createScan, errorMessage, getConfig, getHostMounts } from "../lib/api.js";

  interface Props {
    oncreated: (scanId: string) => void;
  }

  let { oncreated }: Props = $props();

  let rootsText = $state("/");
  let mounts = $state<HostMount[]>([]);
  let hostRoot = $state("/host");
  let error = $state<string | null>(null);
  let busy = $state(false);

  onMount(() => {
    void (async () => {
      try {
        const config = await getConfig();
        if (config.roots.length > 0) {
          rootsText = config.roots.join(", ");
        }
      } catch {
        // Keep the default root when the config cannot be loaded.
      }
      try {
        const response = await getHostMounts();
        mounts = response.mounts;
        hostRoot = response.hostRoot;
      } catch {
        // No host mount available; the user can still type a path.
      }
    })();
  });

  function pickMount(event: Event): void {
    const value = (event.currentTarget as HTMLSelectElement).value;
    if (value) rootsText = value;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    error = null;
    const roots = rootsText
      .split(/[\n,]+/)
      .map((value) => value.trim())
      .filter((value) => value.length > 0);
    if (roots.length === 0) {
      error = "Enter at least one root path.";
      return;
    }
    busy = true;
    try {
      const created = await createScan(roots);
      oncreated(created.id);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<section class="launcher">
  <h2>Start a scan</h2>
  <p class="hint">
    The host filesystem is mounted read-only at <code>{hostRoot}</code>. Pick a drive/mount, or
    type one or more paths (one per line, or comma-separated). Scans are read-only.
  </p>

  {#if mounts.length > 0}
    <label class="mounts">
      <span>Drive / mount on the host</span>
      <select onchange={pickMount} disabled={busy} aria-label="Host drive">
        <option value="">Choose a drive…</option>
        {#each mounts as mount (mount.path)}
          <option value={mount.containerPath}>
            {mount.path}{mount.fstype ? ` · ${mount.fstype}` : ""}{mount.device
              ? ` · ${mount.device}`
              : ""}
          </option>
        {/each}
      </select>
    </label>
  {/if}

  <form onsubmit={submit}>
    <input
      type="text"
      bind:value={rootsText}
      placeholder="{hostRoot}, {hostRoot}/mnt/data"
      aria-label="Root paths"
      disabled={busy}
    />
    <button type="submit" disabled={busy}>{busy ? "Scanning…" : "Scan"}</button>
  </form>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .launcher {
    width: min(420px, 90vw);
    padding: 1.5rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--panel);
  }

  h2 {
    margin: 0 0 0.25rem;
    font-size: 1.1rem;
  }

  .hint {
    margin: 0 0 1rem;
    color: var(--muted);
    font-size: 0.8rem;
  }

  .hint code {
    color: var(--text);
  }

  .mounts {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.75rem;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .mounts select {
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  form {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  form button {
    padding: 0.5rem 0.9rem;
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

  .error {
    margin: 0.75rem 0 0;
    color: var(--err);
    font-size: 0.8rem;
  }

  .progress {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem 1rem;
    margin: 1rem 0 0;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  .progress div {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }

  dt {
    color: var(--muted);
    font-size: 0.75rem;
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  dd.ok {
    color: var(--ok);
  }

  dd.err {
    color: var(--err);
  }
</style>
