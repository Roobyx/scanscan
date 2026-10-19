<script lang="ts">
  import type { DriveInfo, HealthResponse, ScanSummary } from "@scanscan/api-types";

  import { deleteScan, errorMessage, exportUrl, runGc } from "../lib/api.js";
  import { formatAgo, formatBytes } from "../lib/format.js";
  import { scanRootLabel } from "../lib/mounts.js";
  import { stateMeta } from "../lib/scan-state.js";

  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    scans: ScanSummary[];
    selectedId: string | null;
    drives: DriveInfo[];
    health: HealthResponse | null;
    healthError: string | null;
    onopen: (scanId: string) => void | Promise<void>;
    onchanged: () => Promise<void>;
  }

  let { scans, selectedId, drives, health, healthError, onopen, onchanged }: Props = $props();

  let checked = $state<Set<string>>(new Set());
  let busy = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let confirm = $state<"delete" | "gc" | null>(null);

  const selectedScans = $derived(scans.filter((scan) => checked.has(scan.id)));
  const allChecked = $derived(scans.length > 0 && scans.every((scan) => checked.has(scan.id)));
  const completed = $derived(scans.filter((scan) => scan.state === "completed"));

  function timestampOf(scan: ScanSummary): number {
    return scan.finishedAtMs ?? scan.startedAtMs;
  }

  function toggle(id: string): void {
    const next = new Set(checked);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    checked = next;
  }

  function toggleAll(): void {
    checked = allChecked ? new Set<string>() : new Set(scans.map((scan) => scan.id));
  }

  function exportSelected(): void {
    const scan = selectedScans[0];
    if (!scan || scan.state !== "completed") return;
    const anchor = document.createElement("a");
    anchor.href = exportUrl(scan.id, "csv");
    anchor.download = "";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    notice = `Exporting ${scanRootLabel(scan, drives)} as CSV…`;
  }

  async function removeSelected(): Promise<void> {
    if (busy) return;
    const targets = [...selectedScans];
    if (targets.length === 0) return;
    busy = true;
    error = null;
    notice = null;
    try {
      for (const scan of targets) await deleteScan(scan.id);
      checked = new Set<string>();
      await onchanged();
      notice = `Deleted ${targets.length} snapshot${targets.length === 1 ? "" : "s"}.`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
      confirm = null;
    }
  }

  async function collectGarbage(): Promise<void> {
    if (busy) return;
    busy = true;
    error = null;
    notice = null;
    try {
      const result = await runGc(3);
      checked = new Set<string>();
      await onchanged();
      notice = `Garbage collection removed ${result.removed.length} snapshot${
        result.removed.length === 1 ? "" : "s"
      }.`;
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
      confirm = null;
    }
  }

  function formatDate(scan: ScanSummary): string {
    return new Date(timestampOf(scan)).toLocaleString();
  }

  function formatUptime(seconds: number): string {
    if (!Number.isFinite(seconds) || seconds <= 0) return "—";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ${minutes % 60}m`;
    return `${Math.floor(hours / 24)}d ${hours % 24}h`;
  }
</script>

<div class="settings">
  <div class="settings-head">
    <div>
      <h1>Settings</h1>
      <p>Runtime status and snapshot management.</p>
    </div>
  </div>

  <section class="panel">
    <div class="panel-head">
      <div class="title">
        <Icon name="plug" size={15} />
        <h2>Connection</h2>
      </div>
      <span class="muted">runtime status</span>
    </div>
    <div class="connection-grid">
      <article
        class="status-card"
        class:ok={Boolean(health && !healthError)}
        class:err={Boolean(healthError)}
      >
        <div class="status-head">
          <Icon name="plug" size={16} />
          <span>API server</span>
          <span
            class="badge"
            class:ok={Boolean(health && !healthError)}
            class:err={Boolean(healthError)}
          >
            {#if healthError}
              unreachable
            {:else if health}
              {health.status}
            {:else}
              connecting…
            {/if}
          </span>
        </div>
        <p class="status-detail">
          {#if healthError}
            {healthError}
          {:else if health}
            protocol v{health.protocol} · uptime {formatUptime(health.uptimeS)}
          {:else}
            Waiting for the first health response.
          {/if}
        </p>
      </article>

      <article
        class="status-card"
        class:ok={Boolean(health?.core.connected)}
        class:muted={!health?.core.connected}
      >
        <div class="status-head">
          <Icon name="server" size={16} />
          <span>Scan core</span>
          <span
            class="badge"
            class:ok={Boolean(health?.core.connected)}
            class:muted={!health?.core.connected}
          >
            {#if health}
              {health.core.connected ? "connected" : "idle"}
            {:else}
              unknown
            {/if}
          </span>
        </div>
        <p class="status-detail">
          {#if health}
            {health.core.version ? `v${health.core.version} · ` : ""}{health.core.socket}
          {:else}
            No health information yet.
          {/if}
        </p>
      </article>
    </div>
  </section>

  <section class="panel">
    <div class="panel-head">
      <div class="title">
        <Icon name="hard-drive" size={15} />
        <h2>Scan management</h2>
      </div>
      <span class="muted">{scans.length} snapshot{scans.length === 1 ? "" : "s"}</span>
    </div>

    <div class="manage">
      <div class="toolbar">
        <label class="check-all">
          <input
            type="checkbox"
            checked={allChecked}
            disabled={scans.length === 0 || busy}
            onchange={toggleAll}
          />
          <span>Select all</span>
        </label>
        <span class="selected-count">
          {selectedScans.length} selected
        </span>
        <span class="spacer"></span>
        <button
          type="button"
          class="action"
          disabled={selectedScans.length !== 1 || selectedScans[0]?.state !== "completed" || busy}
          title="Select exactly one completed snapshot to export it as CSV"
          onclick={exportSelected}
        >
          <Icon name="download" size={13} />
          Export CSV
        </button>
        <button
          type="button"
          class="action danger"
          disabled={selectedScans.length === 0 || busy}
          onclick={() => (confirm = "delete")}
        >
          <Icon name="trash" size={13} />
          Delete
        </button>
        <button
          type="button"
          class="action"
          disabled={scans.length <= 3 || busy}
          title="Delete all but the 3 most recent snapshots"
          onclick={() => (confirm = "gc")}
        >
          <Icon name="refresh" size={13} />
          GC · keep 3
        </button>
      </div>

      {#if notice}
        <p class="notice" role="status">{notice}</p>
      {/if}
      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}

      {#if scans.length === 0}
        <div class="empty">
          <Icon name="scan" size={26} />
          <p>No snapshots yet. Start one with <strong>+ New scan</strong> in the header.</p>
        </div>
      {:else}
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th class="pick"><span class="sr-only">Select</span></th>
                <th>Root</th>
                <th>State</th>
                <th>Time</th>
                <th class="num">Size</th>
                <th class="num">Items</th>
                <th><span class="sr-only">Actions</span></th>
              </tr>
            </thead>
            <tbody>
              {#each scans as scan (scan.id)}
                {@const meta = stateMeta(scan.state)}
                <tr class:active={scan.id === selectedId}>
                  <td class="pick">
                    <input
                      type="checkbox"
                      checked={checked.has(scan.id)}
                      aria-label="Select {scanRootLabel(scan, drives)}"
                      onchange={() => toggle(scan.id)}
                    />
                  </td>
                  <td>
                    <span class="root">{scanRootLabel(scan, drives)}</span>
                    <span class="path" title={scan.roots.join(", ")}>{scan.roots.join(", ")}</span>
                  </td>
                  <td>
                    <span class="state {meta.tone}">
                      <Icon name={meta.icon} size={11} />
                      {meta.label}
                    </span>
                  </td>
                  <td class="when" title={formatDate(scan)}>{formatAgo(timestampOf(scan))}</td>
                  <td class="num">{formatBytes(scan.bytesAlloc)}</td>
                  <td class="num">
                    {scan.files.toLocaleString()} files · {scan.dirs.toLocaleString()} dirs
                  </td>
                  <td>
                    <div class="row-actions">
                      <button
                        type="button"
                        class="action small"
                        disabled={scan.state !== "completed"}
                        onclick={() => void onopen(scan.id)}
                      >
                        Open
                      </button>
                      {#if scan.state === "completed"}
                        <a
                          class="action small"
                          href={exportUrl(scan.id, "csv")}
                          download
                          title="Export as CSV"
                          aria-label="Export {scanRootLabel(scan, drives)} as CSV"
                        >
                          <Icon name="download" size={13} />
                        </a>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <p class="hint">
        Deleting a snapshot removes its index and manifests. Scanned files are never modified.
        {completed.length} completed snapshot{completed.length === 1 ? "" : "s"} available.
      </p>
    </div>
  </section>
</div>

{#if confirm === "delete"}
  <Modal
    title="Delete snapshots"
    subtitle="Scanned files are never touched."
    onclose={() => {
      if (!busy) confirm = null;
    }}
  >
    <p class="confirm-text">
      Delete {selectedScans.length} snapshot{selectedScans.length === 1 ? "" : "s"}? The index and
      manifests are removed from the data directory.
    </p>
    <div class="modal-actions">
      <button type="button" class="action" disabled={busy} onclick={() => (confirm = null)}>
        Cancel
      </button>
      <button type="button" class="action danger" disabled={busy} onclick={() => void removeSelected()}>
        <Icon name="trash" size={13} />
        {busy ? "Deleting…" : "Delete"}
      </button>
    </div>
  </Modal>
{/if}

{#if confirm === "gc"}
  <Modal
    title="Garbage collection"
    subtitle="Keep the 3 most recent snapshots."
    onclose={() => {
      if (!busy) confirm = null;
    }}
  >
    <p class="confirm-text">
      Delete every snapshot except the 3 most recent? Scanned files are never touched.
    </p>
    <div class="modal-actions">
      <button type="button" class="action" disabled={busy} onclick={() => (confirm = null)}>
        Cancel
      </button>
      <button type="button" class="action danger" disabled={busy} onclick={() => void collectGarbage()}>
        <Icon name="refresh" size={13} />
        {busy ? "Collecting…" : "Run GC"}
      </button>
    </div>
  </Modal>
{/if}

<style>
  .settings {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1rem;
    min-width: 0;
    min-height: 0;
    padding: 1rem;
    overflow: auto;
  }

  .settings-head h1 {
    margin: 0;
    font-size: 1.2rem;
    letter-spacing: 0.01em;
  }

  .settings-head p {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-size: 0.78rem;
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

  .title {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--muted);
  }

  .title h2 {
    margin: 0;
    font-size: 0.85rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .connection-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    gap: 0.75rem;
    padding: 0.85rem;
  }

  .status-card {
    padding: 0.8rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
  }

  .status-card.ok {
    border-color: color-mix(in srgb, var(--ok) 32%, var(--border));
  }

  .status-card.err {
    border-color: color-mix(in srgb, var(--err) 40%, var(--border));
  }

  .status-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.82rem;
    font-weight: 600;
  }

  .status-head svg {
    color: var(--muted);
  }

  .badge {
    margin-left: auto;
    padding: 0.12rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.66rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .badge.ok {
    border-color: color-mix(in srgb, var(--ok) 38%, transparent);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    color: var(--ok);
  }

  .badge.err {
    border-color: color-mix(in srgb, var(--err) 40%, transparent);
    background: color-mix(in srgb, var(--err) 14%, transparent);
    color: var(--err);
  }

  .badge.muted {
    color: var(--muted);
  }

  .status-detail {
    margin: 0.45rem 0 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 0.72rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .manage {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    min-height: 0;
    padding: 0.75rem;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.45rem;
  }

  .check-all {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
    font-size: 0.75rem;
  }

  .selected-count {
    color: var(--muted);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }

  .spacer {
    flex: 1;
  }

  .action {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.32rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 6px;
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

  .action:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .action.danger {
    border-color: color-mix(in srgb, var(--err) 42%, transparent);
    color: var(--err);
  }

  .action.small {
    padding: 0.24rem 0.5rem;
    font-size: 0.7rem;
  }

  .notice,
  .error {
    margin: 0;
    padding: 0.45rem 0.6rem;
    border-radius: 7px;
    font-size: 0.75rem;
  }

  .notice {
    border: 1px solid color-mix(in srgb, var(--ok) 35%, transparent);
    background: color-mix(in srgb, var(--ok) 10%, transparent);
    color: var(--ok);
  }

  .error {
    border: 1px solid color-mix(in srgb, var(--err) 38%, transparent);
    background: color-mix(in srgb, var(--err) 10%, transparent);
    color: var(--err);
  }

  .table-wrap {
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: 9px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
    white-space: nowrap;
  }

  thead th {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--panel-2) 70%, transparent);
    color: var(--muted);
    font-size: 0.66rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  tbody td {
    padding: 0.5rem 0.6rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    vertical-align: middle;
  }

  tbody tr:last-child td {
    border-bottom: none;
  }

  tbody tr.active {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  tbody tr:hover {
    background: color-mix(in srgb, var(--accent) 5%, transparent);
  }

  .pick {
    width: 2rem;
  }

  .root,
  .path {
    display: block;
  }

  .root {
    font-weight: 600;
  }

  .path {
    max-width: 20rem;
    overflow: hidden;
    color: var(--muted);
    font-size: 0.68rem;
    text-overflow: ellipsis;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.12rem 0.42rem;
    border: 1px solid transparent;
    border-radius: 999px;
    font-size: 0.66rem;
    font-weight: 600;
  }

  .state.ok {
    border-color: color-mix(in srgb, var(--ok) 38%, transparent);
    background: color-mix(in srgb, var(--ok) 14%, transparent);
    color: var(--ok);
  }

  .state.info {
    border-color: color-mix(in srgb, var(--accent) 42%, transparent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
  }

  .state.warn {
    border-color: color-mix(in srgb, var(--warn) 40%, transparent);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
    color: var(--warn);
  }

  .state.err {
    border-color: color-mix(in srgb, var(--err) 40%, transparent);
    background: color-mix(in srgb, var(--err) 14%, transparent);
    color: var(--err);
  }

  .state.muted {
    border-color: var(--border);
    color: var(--muted);
  }

  .when {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .num {
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.3rem;
  }

  td:last-child {
    text-align: right;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 2rem 1rem;
    color: var(--muted);
    text-align: center;
  }

  .empty p {
    margin: 0;
    font-size: 0.8rem;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.7rem;
  }

  .muted {
    color: var(--muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    border: 0;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .confirm-text {
    margin: 0 0 1rem;
    color: var(--muted);
    font-size: 0.85rem;
    line-height: 1.5;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  @media (max-width: 760px) {
    .selected-count {
      display: none;
    }

    .toolbar {
      align-items: stretch;
    }

    .action {
      flex: 1;
      justify-content: center;
    }

    .path {
      max-width: 10rem;
    }
  }
</style>
