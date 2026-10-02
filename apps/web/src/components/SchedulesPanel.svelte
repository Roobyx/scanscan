<script lang="ts">
  import { onMount } from "svelte";

  import type { Schedule } from "../lib/api.js";
  import { createSchedule, deleteSchedule, errorMessage, getSchedules } from "../lib/api.js";

  let schedules = $state<Schedule[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let formError = $state<string | null>(null);
  let busy = $state(false);

  let rootsInput = $state("/host");
  let cronInput = $state("");
  let enabledInput = $state(true);

  function parseRoots(value: string): string[] {
    return value
      .split(/[\n,]+/)
      .map((part) => part.trim())
      .filter((part) => part.length > 0);
  }

  function formatTime(ms: number | undefined): string {
    if (ms === undefined || !Number.isFinite(ms)) return "—";
    return new Date(ms).toLocaleString();
  }

  function formatLast(schedule: Schedule): string {
    const when = formatTime(schedule.lastRunMs);
    if (when === "—") return "—";
    return schedule.lastScanId ? `${when} · ${schedule.lastScanId}` : when;
  }

  async function refresh(): Promise<void> {
    loading = true;
    error = null;
    try {
      schedules = await getSchedules();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
  });

  async function handleAdd(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const roots = parseRoots(rootsInput);
    if (roots.length === 0) {
      formError = "Enter at least one root path.";
      return;
    }
    const cron = cronInput.trim();
    const fields = cron.split(/\s+/).filter((field) => field.length > 0);
    if (fields.length !== 5) {
      formError = "Cron must have exactly 5 fields.";
      return;
    }
    formError = null;
    busy = true;
    try {
      await createSchedule({ roots, cron, enabled: enabledInput });
      cronInput = "";
      await refresh();
    } catch (cause) {
      formError = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function handleDelete(id: string): Promise<void> {
    busy = true;
    error = null;
    try {
      await deleteSchedule(id);
      await refresh();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<div class="schedules">
  <form class="form" onsubmit={handleAdd}>
    <div class="fields">
      <label class="field">
        <span class="label">Roots</span>
        <input
          type="text"
          bind:value={rootsInput}
          placeholder="/host, /data"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <label class="field">
        <span class="label">Cron</span>
        <input
          type="text"
          bind:value={cronInput}
          placeholder="0 3 * * *"
          autocomplete="off"
          spellcheck="false"
        />
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={enabledInput} />
        <span>Enabled</span>
      </label>
      <button type="submit" class="add" disabled={busy}>Add</button>
    </div>
    <p class="help">
      Cron format: minute hour day-of-month month day-of-week (5 fields). Example:
      <code>0 3 * * *</code> = daily at 03:00.
    </p>
    {#if formError}
      <p class="state err" role="alert">{formError}</p>
    {/if}
  </form>

  {#if loading && schedules.length === 0}
    <p class="state">Loading schedules…</p>
  {:else if error}
    <p class="state err" role="alert">{error}</p>
  {:else if schedules.length === 0}
    <p class="state">No schedules configured.</p>
  {:else}
    <ul class="list">
      {#each schedules as schedule (schedule.id)}
        <li class="row">
          <div class="row-main">
            <span class="roots" title={schedule.roots.join(", ")}>{schedule.roots.join(", ")}</span>
            <code class="cron">{schedule.cron}</code>
            <span class="status" class:on={schedule.enabled}>
              {schedule.enabled ? "enabled" : "disabled"}
            </span>
            <span class="time">next: {formatTime(schedule.nextRunMs)}</span>
            <span class="time">last: {formatLast(schedule)}</span>
          </div>
          <button
            type="button"
            class="delete"
            disabled={busy}
            onclick={() => void handleDelete(schedule.id)}
          >
            Delete
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .schedules {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    overflow: auto;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .fields {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 0.6rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 12rem;
  }

  .label {
    color: var(--muted);
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .field input {
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
  }

  .field input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding-bottom: 0.35rem;
    color: var(--text);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .add {
    padding: 0.35rem 0.9rem;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, transparent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .add:hover {
    border-color: var(--accent);
  }

  .add:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .help {
    margin: 0;
    color: var(--muted);
    font-size: 0.75rem;
  }

  .help code {
    color: var(--text);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0.75rem;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
  }

  .row-main {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .roots {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.82rem;
  }

  .cron {
    padding: 0.1rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--panel);
    font-size: 0.75rem;
  }

  .status {
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .status.on {
    color: var(--ok);
  }

  .time {
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .delete {
    padding: 0.3rem 0.6rem;
    border: 1px solid color-mix(in srgb, var(--err) 40%, transparent);
    border-radius: 6px;
    background: none;
    color: var(--err);
    font: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .delete:hover {
    border-color: var(--err);
  }

  .delete:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .state {
    margin: 2rem auto;
    color: var(--muted);
    text-align: center;
  }

  .state.err {
    color: var(--err);
  }

  .form .state.err {
    margin: 0;
    text-align: left;
  }
</style>
