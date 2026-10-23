<script lang="ts">
  import type { DriveInfo, DriveStats, ScanSummary } from "@scanscan/api-types";
  import { createScan, errorMessage } from "../lib/api.js";
  import { formatAgo, formatBytes, formatCount, percent } from "../lib/format.js";
  import { isSystemMount, mountLabel, orderMounts } from "../lib/mounts.js";
  import { usageColor } from "../lib/color.js";

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

  interface Fact {
    label: string;
    value: string;
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
    temperatureLabel: string;
    temperatureClass: string;
    facts: Fact[];
  }

  type CardTab = "usage" | "details";

  let { scans, drives, hostRoot, warning, error, loading, onrefresh, onopen, oncreated }: Props =
    $props();

  let scanning = $state<string | null>(null);
  let scanError = $state<string | null>(null);
  let tabs = $state<Record<string, CardTab>>({});

  function tabOf(key: string): CardTab {
    return tabs[key] ?? "usage";
  }

  function setTab(key: string, tab: CardTab): void {
    tabs = { ...tabs, [key]: tab };
  }

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

  function modelOf(stats: DriveStats): string {
    const vendor = stats.vendor?.trim() ?? "";
    const model = stats.model?.trim() ?? "";
    const usefulVendor = vendor === "ATA" ? "" : vendor;
    if (usefulVendor.length > 0 && model.length > 0 && !model.startsWith(usefulVendor)) {
      return `${usefulVendor} ${model}`;
    }
    return model || usefulVendor;
  }

  function diskKind(stats: DriveStats): string {
    if (stats.rotational === true) return "HDD";
    if (stats.rotational === false) return "SSD";
    return "";
  }

  /** Rows for the details tab; absent counters are simply not rendered. */
  function factsOf(drive: DriveInfo): Fact[] {
    const stats = drive.stats;
    if (!stats) return [];
    const facts: Fact[] = [];

    const model = modelOf(stats);
    if (model) facts.push({ label: "Model", value: model });
    if (stats.serial) facts.push({ label: "Serial", value: stats.serial });
    if (stats.kernelName) facts.push({ label: "Device", value: stats.kernelName });
    if (stats.backingDevices && stats.backingDevices.length > 0) {
      facts.push({ label: "Backed by", value: stats.backingDevices.join(", ") });
    }

    const kind = diskKind(stats);
    if (kind) facts.push({ label: "Type", value: kind });
    if (stats.transport) facts.push({ label: "Bus", value: stats.transport });
    if (drive.fstype) facts.push({ label: "Filesystem", value: drive.fstype });
    if (stats.logicalBlockSize !== undefined) {
      const physical = stats.physicalBlockSize;
      const size =
        physical !== undefined && physical !== stats.logicalBlockSize
          ? `${stats.logicalBlockSize} / ${physical} B`
          : `${stats.logicalBlockSize} B`;
      facts.push({ label: "Block size", value: size });
    }

    if (stats.inodesTotal !== undefined && stats.inodesTotal > 0) {
      const usedInodes =
        stats.inodesUsed ?? Math.max(0, stats.inodesTotal - (stats.inodesFree ?? 0));
      const pct = percent(usedInodes, stats.inodesTotal).toFixed(0);
      facts.push({
        label: "Inodes",
        value: `${formatCount(usedInodes)} of ${formatCount(stats.inodesTotal)} (${pct}%)`,
      });
    }

    if (stats.bytesRead !== undefined) {
      const ops = formatCount(stats.readsCompleted ?? 0);
      facts.push({
        label: "Read",
        value: `${formatBytes(stats.bytesRead)} · ${ops} ops`,
      });
    }
    if (stats.bytesWritten !== undefined) {
      const ops = formatCount(stats.writesCompleted ?? 0);
      facts.push({
        label: "Written",
        value: `${formatBytes(stats.bytesWritten)} · ${ops} ops`,
      });
    }
    if (stats.busyPct !== undefined) {
      facts.push({ label: "Busy", value: `${stats.busyPct.toFixed(0)}% since boot` });
    }
    if (stats.ioInProgress !== undefined && stats.ioInProgress > 0) {
      facts.push({ label: "In flight", value: String(stats.ioInProgress) });
    }

    return facts;
  }

  function tempClassOf(celsius: number | undefined): string {
    if (celsius === undefined) return "";
    if (celsius >= 70) return "hot";
    if (celsius >= 55) return "warm";
    return "";
  }

  const cards = $derived.by(() => {
    return orderMounts(drives).map((drive) => {
      const total = drive.totalBytes ?? 0;
      const free = drive.freeBytes ?? 0;
      const used = drive.usedBytes ?? Math.max(0, total - free);
      const scan = scanFor(drive.containerPath, scans);
      const scanDisabled =
        scanning === drive.containerPath || scanRunning(drive.containerPath, scans);
      const temperatureC = drive.stats?.temperatureC;
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
        temperatureLabel: temperatureC === undefined ? "" : `${temperatureC.toFixed(0)}°C`,
        temperatureClass: tempClassOf(temperatureC),
        facts: factsOf(drive),
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
            {#if card.temperatureLabel}
              <span
                class="pill temp {card.temperatureClass}"
                title="Temperature reported by the host hwmon sensor"
              >
                <Icon name="thermometer" size={11} />
                {card.temperatureLabel}
              </span>
            {/if}
          </span>
        </div>

        <div class="tabs" role="group" aria-label="{card.label} details">
          <button
            type="button"
            class="tab"
            class:on={tabOf(card.key) === "usage"}
            aria-pressed={tabOf(card.key) === "usage"}
            onclick={() => setTab(card.key, "usage")}
          >
            Usage
          </button>
          <button
            type="button"
            class="tab"
            class:on={tabOf(card.key) === "details"}
            aria-pressed={tabOf(card.key) === "details"}
            onclick={() => setTab(card.key, "details")}
          >
            Details
          </button>
        </div>

        {#if tabOf(card.key) === "usage"}
          <div class="panel">
            {#if card.hasCapacity}
              <div class="meter" role="img" aria-label="{card.usedPct.toFixed(0)} percent used">
                <span
                  class="fill"
                  style="width: {card.usedPct}%; background: {usageColor(card.usedPct)}"
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
          </div>
        {:else}
          <div class="panel">
            {#if card.facts.length > 0}
              <dl class="facts">
                {#each card.facts as fact (fact.label)}
                  <dt>{fact.label}</dt>
                  <dd>{fact.value}</dd>
                {/each}
              </dl>
              {#if card.drive.stats?.bytesRead !== undefined}
                <p class="note">I/O counters are cumulative since boot.</p>
              {/if}
            {:else}
              <p class="empty">No hardware counters are exposed for this mount.</p>
            {/if}
          </div>
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
    min-width: 0;
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
    flex-wrap: wrap;
    justify-content: flex-end;
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

  .pill.temp {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    font-variant-numeric: tabular-nums;
  }

  .pill.temp.warm {
    border-color: color-mix(in srgb, var(--warn) 35%, var(--border));
    color: var(--warn);
  }

  .pill.temp.hot {
    border-color: color-mix(in srgb, var(--err) 40%, var(--border));
    color: var(--err);
  }

  .tabs {
    display: inline-flex;
    align-self: flex-start;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: color-mix(in srgb, var(--border) 20%, transparent);
  }

  .tab {
    padding: 0.16rem 0.55rem;
    border: 0;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.7rem;
    line-height: 1.4;
    cursor: pointer;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: 1px;
  }

  .tab.on {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--text);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-height: 3.9rem;
  }

  .facts {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 0.2rem 0.6rem;
    margin: 0;
    font-size: 0.72rem;
  }

  .facts dt {
    color: var(--muted);
  }

  .facts dd {
    margin: 0;
    overflow-wrap: anywhere;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .drive.system .facts dd {
    color: var(--muted);
  }

  .note,
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 0.7rem;
  }

  .note {
    opacity: 0.8;
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
