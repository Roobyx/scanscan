import type { ScanState } from "@scanscan/api-types";

import type { IconName } from "./icons.js";

export interface StateMeta {
  label: string;
  tone: "ok" | "info" | "warn" | "err" | "muted";
  icon: IconName;
}

const META: Record<ScanState, StateMeta> = {
  queued: { label: "Queued", tone: "muted", icon: "clock" },
  running: { label: "Running", tone: "info", icon: "loader" },
  completed: { label: "Completed", tone: "ok", icon: "check-circle" },
  failed: { label: "Failed", tone: "err", icon: "x-circle" },
  cancelled: { label: "Cancelled", tone: "warn", icon: "ban" },
};

export function stateMeta(state: ScanState): StateMeta {
  return META[state];
}
