<script lang="ts">
  import { onMount } from "svelte";

  import type { HostMount } from "@scanscan/api-types";
  import { createScan, errorMessage, getConfig, getHostMounts } from "../lib/api.js";
  import { isSystemMount, mountLabel, orderMounts } from "../lib/mounts.js";

  import Icon from "./Icon.svelte";
  import Spinner from "./Spinner.svelte";

  interface Props {
    oncreated: (scanId: string) => void;
  }

  let { oncreated }: Props = $props();

  let rootsText = $state("/");
  let mounts = $state<HostMount[]>([]);
  let hostRoot = $state("/host");
  let excludeText = $state("");
  let crossFs = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const orderedMounts = $derived(orderMounts(mounts));

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

  function pickMount(containerPath: string): void {
    rootsText = containerPath;
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
      const exclusions = excludeText
        .split(/[\n,]+/)
        .map((value) => value.trim())
        .filter((value) => value.length > 0);
      const created = await createScan(roots, { oneFileSystem: !crossFs, exclusions });
      oncreated(created.id);
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<section class="launcher">
  <p class="hint">
    The host filesystem is mounted read-only at <code>{hostRoot}</code>. Pick a drive or mount, or
    type one or more paths (one per line, or comma-separated). Scans are read-only.
  </p>

  {#if orderedMounts.length > 0}
    <fieldset class="mounts" disabled={busy}>
      <legend>Drive / mount on the host</legend>
      <ul class="mount-list">
        {#each orderedMounts as mount (mount.path)}
          {@const system = isSystemMount(mount.path)}
          {@const picked = rootsText === mount.containerPath}
          <li class:system={system}>
            <button
              type="button"
              class="mount-row"
              class:selected={picked}
              aria-pressed={picked}
              onclick={() => pickMount(mount.containerPath)}
            >
              <span class="mount-icon">
                <Icon name="hard-drive" size={16} />
              </span>
              <span class="mount-main">
                <span class="mount-name">{mountLabel(mount.path)}</span>
                <span class="mount-meta" title="{mount.device} {mount.fstype}">
                  {mount.fstype || "unknown filesystem"}
                  {mount.device ? ` · ${mount.device}` : ""}
                </span>
              </span>
              <span class="mount-side">
                {#if system}
                  <span class="sys-badge">System mount point</span>
                {/if}
                {#if picked}
                  <span class="picked" aria-label="Selected">
                    <Icon name="check" size={13} />
                  </span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </fieldset>
  {/if}

  <form onsubmit={submit}>
    <div class="field">
      <label for="scan-roots">Root paths</label>
      <div class="path-row">
        <input
          id="scan-roots"
          type="text"
          bind:value={rootsText}
          placeholder="{hostRoot}, {hostRoot}/mnt/data"
          disabled={busy}
        />
        <button type="submit" disabled={busy}>
          <Icon name="scan" size={14} />
          {busy ? "Scanning…" : "Scan"}
        </button>
      </div>
    </div>

    <label class="cross">
      <input type="checkbox" bind:checked={crossFs} disabled={busy} />
      <span>Include other mounted filesystems (otherwise it stops at mount boundaries)</span>
    </label>

    <div class="field">
      <label for="scan-exclusions">Exclude paths (globs, one per line or comma-separated)</label>
      <input
        id="scan-exclusions"
        type="text"
        bind:value={excludeText}
        placeholder="{hostRoot}/mnt/piDrive, {hostRoot}/mnt/backupDrive"
        disabled={busy}
      />
    </div>
  </form>

  {#if busy}
    <p class="busy" role="status" aria-label="Starting the scan">
      <Spinner size={12} label="Starting the scan" />
      <span>Starting the scan…</span>
    </p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .launcher {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .hint code {
    color: var(--text);
  }

  .mounts {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: none;
  }

  .mounts legend,
  .field label {
    display: block;
    margin-bottom: 0.35rem;
    color: var(--muted);
    font-size: 0.72rem;
    letter-spacing: 0.03em;
  }

  .mount-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 15rem;
    margin: 0;
    padding: 0.3rem;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: color-mix(in srgb, var(--bg) 55%, var(--panel));
    list-style: none;
  }

  .mount-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.5rem 0.55rem;
    border: 1px solid transparent;
    border-radius: 7px;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .mount-row:hover:not(:disabled) {
    border-color: var(--border);
    background: var(--panel-2);
  }

  .mount-row.selected {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .mount-row:disabled {
    cursor: default;
  }

  .mount-icon {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 1.9rem;
    height: 1.9rem;
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--accent);
  }

  .mount-main {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }

  .mount-name {
    overflow: hidden;
    font-size: 0.82rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mount-meta {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.68rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mount-side {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 0.4rem;
    margin-left: auto;
  }

  .sys-badge {
    padding: 0.12rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.64rem;
    white-space: nowrap;
  }

  .picked {
    display: inline-flex;
    align-items: center;
    color: var(--accent);
  }

  .mount-list li.system .mount-row {
    color: var(--muted);
    background: color-mix(in srgb, var(--panel-2) 45%, transparent);
  }

  .mount-list li.system .mount-icon {
    background: transparent;
    color: var(--muted);
    opacity: 0.7;
  }

  .mount-list li.system .mount-name {
    color: var(--muted);
    font-weight: 500;
  }

  .mount-list li.system .mount-row:hover:not(:disabled),
  .mount-list li.system .mount-row.selected {
    background: color-mix(in srgb, var(--panel-2) 75%, transparent);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .field {
    display: flex;
    flex-direction: column;
  }

  .path-row {
    display: flex;
    gap: 0.5rem;
  }

  input[type="text"] {
    flex: 1;
    min-width: 0;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  input[type="text"]:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .path-row button {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 0.35rem;
    padding: 0.5rem 0.9rem;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--accent);
    color: #0b0e14;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .path-row button:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .cross {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .busy {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .error {
    margin: 0;
    color: var(--err);
    font-size: 0.8rem;
  }

  @media (max-width: 560px) {
    .path-row {
      flex-direction: column;
    }

    .path-row button {
      justify-content: center;
    }
  }
</style>
