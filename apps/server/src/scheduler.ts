import type { CoreApi } from "./core/client.js";

/** A scheduled scan definition. */
export interface Schedule {
  id: string;
  roots: string[];
  /** 5-field cron expression: minute hour day-of-month month day-of-week. */
  cron: string;
  enabled: boolean;
  lastRunMs?: number;
  nextRunMs?: number;
  lastScanId?: string;
}

function parseField(field: string, min: number, max: number): (value: number) => boolean {
  const checks: Array<(value: number) => boolean> = [];
  for (const part of field.split(",")) {
    if (part === "*") {
      checks.push(() => true);
    } else if (/^\*\/\d+$/.test(part)) {
      const step = Number(part.slice(2));
      if (step > 0) checks.push((value) => value % step === 0);
    } else if (/^\d+-\d+$/.test(part)) {
      const [a, b] = part.split("-").map(Number);
      if (a !== undefined && b !== undefined) checks.push((value) => value >= a && value <= b);
    } else if (/^\d+$/.test(part)) {
      const n = Number(part);
      checks.push((value) => value === n);
    }
  }
  if (checks.length === 0) checks.push((value) => value >= min && value <= max);
  return (value) => checks.some((check) => check(value));
}

/** True when `date` (to the minute) matches a 5-field cron expression. */
export function cronMatches(expr: string, date: Date): boolean {
  const parts = expr.trim().split(/\s+/);
  if (parts.length !== 5) return false;
  const [min, hour, dom, month, dow] = parts as [string, string, string, string, string];
  return (
    parseField(min, 0, 59)(date.getMinutes()) &&
    parseField(hour, 0, 23)(date.getHours()) &&
    parseField(dom, 1, 31)(date.getDate()) &&
    parseField(month, 1, 12)(date.getMonth() + 1) &&
    parseField(dow, 0, 6)(date.getDay())
  );
}

function nextRun(cron: string, fromMs: number): number {
  const date = new Date(fromMs);
  date.setSeconds(0, 0);
  for (let i = 0; i < 60 * 24 * 366; i += 1) {
    date.setMinutes(date.getMinutes() + 1);
    if (cronMatches(cron, date)) return date.getTime();
  }
  return fromMs + 3_600_000;
}

/**
 * In-memory scan scheduler. Ticks every 30s and creates a scan for each due
 * schedule. Schedules are not persisted yet (a catalog lands later).
 */
export class Scheduler {
  private schedules = new Map<string, Schedule>();
  private timer?: ReturnType<typeof setInterval>;

  constructor(private readonly core: CoreApi) {}

  list(): Schedule[] {
    return [...this.schedules.values()];
  }

  add(input: { roots: string[]; cron: string; enabled?: boolean }): Schedule {
    const id = `sched_${Math.random().toString(36).slice(2, 10)}`;
    const schedule: Schedule = {
      id,
      roots: input.roots,
      cron: input.cron,
      enabled: input.enabled ?? true,
      nextRunMs: nextRun(input.cron, Date.now()),
    };
    this.schedules.set(id, schedule);
    return schedule;
  }

  remove(id: string): boolean {
    return this.schedules.delete(id);
  }

  start(): void {
    if (this.timer) return;
    this.timer = setInterval(() => void this.tick(), 30_000);
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
  }

  private async tick(): Promise<void> {
    const now = Date.now();
    for (const schedule of this.schedules.values()) {
      if (!schedule.enabled || !schedule.nextRunMs || now < schedule.nextRunMs) continue;
      try {
        const summary = await this.core.call<{ id: string }>("scans.create", {
          roots: schedule.roots,
        });
        schedule.lastRunMs = now;
        schedule.lastScanId = summary.id;
      } catch {
        // A failed trigger should not stop the scheduler.
      }
      schedule.nextRunMs = nextRun(schedule.cron, now + 60_000);
    }
  }
}
