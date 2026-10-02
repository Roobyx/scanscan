<script lang="ts">
  import type { DiffEntry, DiffResult } from "../lib/api.js";
  import { errorMessage, getDiff } from "../lib/api.js";
  import { formatBytes } from "../lib/format.js";

  interface ScanOption {
    id: string;
    roots: string[];
    state: string;
    startedAtMs: number;
  }

  interface Props {
    scanId: string;
    scans: ScanOption[];
  }

  interface Section {
    title: string;
    entries: DiffEntry[];
    compare: boolean;
    sizeKey: "before" | "after";
    sign: 1 | -1;
  }

  const ROW_LIMIT = 100;

  let { scanId, scans }: Props = $props();

  let otherId = $state("");
  let result = $state<DiffResult | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let seq = 0;

  const candidates = $derived(
    scans
      .filter((scan) => scan.id !== scanId && scan.state === "completed")
      .sort((a, b) => b.startedAtMs - a.startedAtMs),
  );

  function buildSections(data: DiffResult): Section[] {
    return [
      { title: "Grown", entries: data.grown, compare: true, sizeKey: "after", sign: 1 },
      { title: "Shrunk", entries: data.shrunk, compare: true, sizeKey: "after", sign: -1 },
      { title: "Added", entries: data.added, compare: false, sizeKey: "after", sign: 1 },
      { title: "Removed", entries: data.removed, compare: false, sizeKey: "before", sign: -1 },
    ];
  }

  const sections: Section[] = $derived(result ? buildSections(result) : []);

  async function load(current: string, other: string): Promise<void> {
    const ticket = ++seq;
    loading = true;
    error = null;
    try {
      const next = await getDiff(current, other);
      if (ticket !== seq) return;
      result = next;
    } catch (cause) {
      if (ticket !== seq) return;
      error = errorMessage(cause);
      result = null;
    } finally {
      if (ticket === seq) loading = false;
    }
  }

  let trackedScan = "";

  $effect(() => {
    if (scanId === trackedScan) return;
    trackedScan = scanId;
    otherId = "";
    result = null;
    error = null;
    const first = candidates[0];
    if (first) otherId = first.id;
  });

  $effect(() => {
    const current = scanId;
    const other = otherId;
    if (current.length === 0 || other.length === 0) return;
    void load(current, other);
  });

  function deltaLabel(entry: DiffEntry, sign: 1 | -1): string {
    const signed = entry.delta * sign;
    return `${signed > 0 ? "+" : "−"}${formatBytes(Math.abs(signed))}`;
  }
</script>

<div class="diff">
  <div class="toolbar">
    <label>
      <span>Baseline</span>
      <select bind:value={otherId} aria-label="Baseline scan">
        <option value="" disabled>Select baseline</option>
        {#each candidates as scan (scan.id)}
          <option value={scan.id}>{scan.roots.join(", ") || scan.id}</option>
        {/each}
      </select>
    </label>
    {#if loading && result}
      <span class="muted">Comparing…</span>
    {/if}
  </div>

  {#if candidates.length === 0}
    <p class="state">No other completed scans to compare against.</p>
  {:else if otherId.length === 0}
    <p class="state">Select a baseline scan to compare.</p>
  {:else if loading && !result}
    <p class="state">Comparing…</p>
  {:else if error}
    <p class="state err" role="alert">{error}</p>
  {:else if result}
    <dl class="totals">
      <div>
        <dt>Before</dt>
        <dd>{formatBytes(result.totals.before)}</dd>
      </div>
      <div>
        <dt>After</dt>
        <dd>{formatBytes(result.totals.after)}</dd>
      </div>
      <div>
        <dt>Δ</dt>
        <dd class:pos={result.totals.delta > 0} class:neg={result.totals.delta < 0}>
          {result.totals.delta > 0 ? "+" : result.totals.delta < 0 ? "−" : ""}{formatBytes(
            Math.abs(result.totals.delta),
          )}
        </dd>
      </div>
      <div>
        <dt>Added</dt>
        <dd class="pos">+{formatBytes(result.totals.added)}</dd>
      </div>
      <div>
        <dt>Removed</dt>
        <dd class="neg">−{formatBytes(result.totals.removed)}</dd>
      </div>
    </dl>

    {#each sections as section (section.title)}
      <section class="block">
        <h3>
          {section.title}
          <span class="muted">{section.entries.length.toLocaleString()}</span>
        </h3>
        {#if section.entries.length === 0}
          <p class="muted small">None.</p>
        {:else}
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Path</th>
                  {#if section.compare}
                    <th class="num">Before</th>
                    <th class="num">After</th>
                    <th class="num">Δ</th>
                  {:else}
                    <th class="num">Size</th>
                  {/if}
                </tr>
              </thead>
              <tbody>
                {#each section.entries.slice(0, ROW_LIMIT) as entry (entry.path)}
                  <tr>
                    <td class="ellipsis" title={entry.path}>{entry.path}</td>
                    {#if section.compare}
                      <td class="num">{formatBytes(entry.before)}</td>
                      <td class="num">{formatBytes(entry.after)}</td>
                      <td
                        class="num"
                        class:pos={section.sign > 0}
                        class:neg={section.sign < 0}
                      >
                        {deltaLabel(entry, section.sign)}
                      </td>
                    {:else}
                      <td class="num">{formatBytes(entry[section.sizeKey])}</td>
                    {/if}
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if section.entries.length > ROW_LIMIT}
              <p class="muted small">
                Showing first {ROW_LIMIT} of {section.entries.length.toLocaleString()}.
              </p>
            {/if}
          </div>
        {/if}
      </section>
    {/each}
  {/if}
</div>

<style>
  .diff {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
  }

  .toolbar {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .toolbar label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .toolbar label span {
    color: var(--muted);
    font-size: 0.68rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  select {
    max-width: 22rem;
    padding: 0.3rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
  }

  .totals {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(7rem, 1fr));
    gap: 0.5rem;
    margin: 0;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .totals div {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .totals dt {
    color: var(--muted);
    font-size: 0.65rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .totals dd {
    margin: 0;
    font-size: 0.9rem;
    font-variant-numeric: tabular-nums;
  }

  .block {
    padding: 0.75rem;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 55%, transparent);
  }

  h3 {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    margin: 0 0 0.5rem;
    color: var(--muted);
    font-size: 0.8rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
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
    max-width: 24rem;
    padding: 0.3rem 0.5rem;
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
    white-space: nowrap;
  }

  .pos {
    color: var(--warn);
  }

  .neg {
    color: var(--ok);
  }

  .small {
    font-size: 0.78rem;
  }

  .state {
    margin: 2rem auto;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }

  .muted {
    color: var(--muted);
  }
</style>
