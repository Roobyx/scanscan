<script lang="ts">
  import type { DriveInfo, ScanSummary } from "@scanscan/api-types";
  import { createScan, errorMessage } from "../lib/api.js";
  import { formatAgo, formatBytes, percent } from "../lib/format.js";
  import { isSystemMount, mountLabel, orderMounts } from "../lib/mounts.js";

  import Icon from "./Icon.svelte";
  import Spinner from "./Spinner.svelte";

  interface Props {
    scans: ScanSummary[];
    drives: DriveInfo[];
    hostRoot: string;
    warning: string | null;
    error: string | null;
    loading: boolean;
    onrefresh: () => void;
    onopen: (scanId: string) => void;
    oncreated: (scanId: string) => void;
  }

  interface Card {
    key: string;
    drive: DriveInfo;
    label: string;
    system: boolean;
    total: number;
    free: number;
    used: number;
    usedPct: number;
    hasCapacity: boolean;
    scannId: string | null;
    scannBytes: number;
    scannAgo: string;
    scanDisabled: boolean;
    scanLabel: string;
  }

  let { scans, drives, hostRoot, warning, error, loading, onrefresh, onopen, oncreated }: Props =
    $props();

  let scanning = $state<string | null>(null);
  let scanError = $state<string | null>(null);

  function timestampOf(scan: ScanSummary): number {
    return scan.finishedAtMs ?? scan.startedAtMs;
  }

  /** Newest completed snapshot whose root is exactly this drive. */
  function scanFor(containerPath: string, all: ScanSummary[]): ScanSummary | null {
    let found: ScanSummary | null = null;
    for (const scan of all) {
      if (scan.state !== "completed") continue;
      if (!scan.roots.includes(containerPath)) continue;
      if (!found || timestampOf(scan) > timestampOf(found)) found = scan;
    }
    return found;
  }

  /** True while a scan of this filesystem is in flight, whoever started it. */
  function scanRunning(containerPath: string, all: ScanSummary[]): boolean {
    return all.some(
      (scan) => scan.state === "running" && scan.roots.includes(containerPath),
    );
  }

  const cards = $derived.by(() => {
    return orderMounts(drives).map((drive) => {
      const total = drive.totalBytes ?? 0;
      const free = drive.freeBytes ?? 0;
      const used = drive.usedBytes ?? Math.max(0, total - free);
      const scan = scanFor(drive.containerPath, scans);
      const scanDisabled =
        scanning === drive.containerPath || scanRunning(drive.containerPath, scans);
      return {
        key: drive.containerPath,
        drive,
        label: mountLabel(drive.path),
        system: isSystemMount(drive.path),
        total,
        free,
        used,
        usedPct: percent(used, total),
        hasCapacity: total > 0,
        scannId: scan ? scan.id : null,
        scannBytes: scan ? scan.bytesAlloc : 0,
        scannAgo: scan ? formatAgo(timestampOf(scan)) : "",
        scanDisabled,
        scanLabel: scanDisabled ? "Scanning…" : scan ? "Rescan" : "Scan",
      } as Card;
    });
  });

  async function startScan(card: Card): Promise<void> {
    if (scanning || card.scanDisabled) return;
    scanError = null;
    scanning = card.drive.containerPath;
    try {
      const created = await createScan([card.drive.containerPath], { oneFileSystem: true });
      oncreated(created.id);
    } catch (cause) {
      scanError = errorMessage(cause);
    } finally {
      scanning = null;
    }
  }
</script>

<div class="drives">
  <div class="head">
    <p class="hint">
      {#if loading}
        <span class="busy" role="status" aria-label="Reading host filesystems">
          <Spinner size={12} label="Reading host filesystems" />
          <span>Reading host filesystems…</span>
        </span>
      {:else if drives.length === 0}
        No host filesystem is visible at <code>{hostRoot}</code>. Mount a drive read-only there
        and reload.
      {:else}
        {drives.length} filesystems mounted read-only under <code>{hostRoot}</code>
      {/if}
    </p>
    <button type="button" class="action" disabled={loading} onclick={onrefresh}>
      Refresh
    </button>
  </div>

  {#if warning}
    <p class="warn">
      Core unreachable ({warning}) — only the configured root is listed.
    </p>
  {/if}

  {#if error}
    <p class="err" role="alert">{error}</p>
  {/if}

  {#if scanError}
    <p class="err" role="alert">{scanError}</p>
  {/if}

  <ul class="grid">
    {#each cards as card (card.key)}
      <li class="drive" class:system={card.system}>
        <div class="drive-head">
          <span class="drive-name" title={card.drive.path}>
            <Icon name="hard-drive" size={15} />
            {card.label}
          </span>
          <span class="tags">
            {#if card.system}
              <span class="pill system">System mount point</span>
            {/if}
            {#if card.drive.fstype}
              <span class="pill">{card.drive.fstype}</span>
            {/if}
          </span>
        </div>

        {#if card.hasCapacity}
          <div class="meter" role="img" aria-label="{card.usedPct.toFixed(0)} percent used">
            <span
              class="fill"
              class:tight={card.usedPct >= 90}
              class:critical={card.usedPct >= 97}
              style="width: {card.usedPct}%"
            ></span>
          </div>
          <p class="cap">
            <strong>{formatBytes(card.free)}</strong> free of {formatBytes(card.total)}
          </p>
          <p class="meta">{formatBytes(card.used)} used · {card.usedPct.toFixed(0)}%</p>
        {:else}
          <div class="meter unknown"></div>
          <p class="cap">Capacity unavailable</p>
          <p class="meta">the host reports no filesystem stats for this mount</p>
        {/if}

        <p class="scann">
          {#if card.scannId}
            {formatBytes(card.scannBytes)} scann · {card.scannAgo}
          {:else}
            never scann
          {/if}
        </p>

        <div class="actions">
          {#if card.scannId}
            <button type="button" class="action" onclick={() => onopen(card.scannId ?? "")}>
              Open
            </button>
          {/if}
          <button
            type="button"
            class="action primary"
            disabled={card.scanDisabled}
            onclick={() => void startScan(card)}
          >
            {card.scanLabel}
          </button>
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .drives {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
    padding: 1rem;
    overflow: auto;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.8rem;
  }

  .hint code {
    color: var(--text);
  }

  .busy {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
  }

  .warn {
    margin: 0;
    color: var(--warn);
    font-size: 0.78rem;
  }

  .err {
    margin: 0;
    color: var(--err);
    font-size: 0.78rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
    gap: 0.75rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .drive {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .drive.system {
    background: color-mix(in srgb, var(--panel-2) 62%, var(--panel));
    color: var(--muted);
  }

  .drive.system .drive-name {
    color: var(--muted);
    font-weight: 500;
  }

  .drive.system .meter .fill {
    opacity: 0.65;
  }

  .drive-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .drive-name {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    overflow: hidden;
    font-size: 1rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .drive-name svg {
    flex: none;
    color: var(--muted);
  }

  .tags {
    display: inline-flex;
    flex: none;
    gap: 0.3rem;
  }

  .pill {
    padding: 0.1rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-size: 0.7rem;
    white-space: nowrap;
  }

  .pill.system {
    border-color: color-mix(in srgb, var(--warn) 30%, var(--border));
    color: color-mix(in srgb, var(--muted) 80%, var(--warn));
  }

  .meter {
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: color-mix(in srgb, var(--border) 70%, transparent);
  }

  .meter .fill {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }

  .meter .fill.tight {
    background: var(--warn);
  }

  .meter .fill.critical {
    background: var(--err);
  }

  .meter.unknown {
    opacity: 0.4;
    background: var(--border);
  }

  .cap {
    margin: 0;
    font-size: 0.9rem;
  }

  .cap strong {
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
  }

  .meta,
  .scann {
    margin: 0;
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    gap: 0.4rem;
    margin-top: auto;
    padding-top: 0.25rem;
  }

  .action {
    padding: 0.3rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .action:hover {
    border-color: var(--accent);
  }

  .action:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .action.primary {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
</style>
