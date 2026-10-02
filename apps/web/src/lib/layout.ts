import type { HierarchyNode } from "@scanscan/api-types";

/** One radial arc produced by {@link sunburstArcs}. */
export interface SunburstArc {
  node: HierarchyNode;
  depth: number;
  angle: number;
  path: string;
  labelX: number;
  labelY: number;
}

export interface SunburstOptions {
  size?: number;
  innerRadius?: number;
  rings?: number;
}

function polar(cx: number, cy: number, radius: number, angle: number): [number, number] {
  return [cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius];
}

/** SVG path for an annulus sector; `r0 === 0` yields a pie slice. */
export function annulusPath(
  cx: number,
  cy: number,
  r0: number,
  r1: number,
  a0: number,
  a1: number,
): string {
  const large = a1 - a0 > Math.PI ? 1 : 0;
  const [x0o, y0o] = polar(cx, cy, r1, a0);
  const [x1o, y1o] = polar(cx, cy, r1, a1);
  if (r0 <= 0) {
    return `M ${cx} ${cy} L ${x0o} ${y0o} A ${r1} ${r1} 0 ${large} 1 ${x1o} ${y1o} Z`;
  }
  const [x1i, y1i] = polar(cx, cy, r0, a1);
  const [x0i, y0i] = polar(cx, cy, r0, a0);
  return `M ${x0o} ${y0o} A ${r1} ${r1} 0 ${large} 1 ${x1o} ${y1o} L ${x1i} ${y1i} A ${r0} ${r0} 0 ${large} 0 ${x0i} ${y0i} Z`;
}

/**
 * Concentric arcs for a sunburst. Only `root`'s descendants are emitted; the
 * root itself becomes the centre hole. Angle is proportional to `size` among
 * siblings.
 */
export function sunburstArcs(root: HierarchyNode, options: SunburstOptions = {}): SunburstArc[] {
  const size = options.size ?? 640;
  const rings = Math.max(1, options.rings ?? 2);
  const cx = size / 2;
  const cy = size / 2;
  const outer = size / 2 - 6;
  const inner = options.innerRadius ?? Math.max(24, outer * 0.24);
  const thickness = Math.max(1, (outer - inner) / rings);
  const arcs: SunburstArc[] = [];

  const place = (
    nodes: HierarchyNode[],
    depth: number,
    from: number,
    extent: number,
    r0: number,
  ): void => {
    if (depth >= rings || nodes.length === 0 || extent <= 0) return;
    const sum = nodes.reduce((acc, node) => acc + Math.max(0, node.size), 0);
    if (sum <= 0) return;
    const r1 = Math.min(outer, r0 + thickness);
    let angle = from;
    for (const node of nodes) {
      const a0 = angle;
      const a1 = angle + (Math.max(0, node.size) / sum) * extent;
      angle = a1;
      if (a1 - a0 < 1e-4) continue;
      const mid = (a0 + a1) / 2;
      const rMid = (r0 + r1) / 2;
      arcs.push({
        node,
        depth: depth + 1,
        angle: a1 - a0,
        path: annulusPath(cx, cy, r0, r1, a0, a1),
        labelX: cx + Math.cos(mid) * rMid,
        labelY: cy + Math.sin(mid) * rMid,
      });
      const kids = node.children ?? [];
      if (kids.length > 0) place(kids, depth + 1, a0, a1 - a0, r1);
    }
  };

  place(root.children ?? [], 0, -Math.PI / 2, Math.PI * 2, inner);
  return arcs;
}

/** One horizontal cell produced by {@link icicleRects}; `x`/`w` are 0..1. */
export interface IcicleRect {
  node: HierarchyNode;
  depth: number;
  x: number;
  w: number;
}

/** Rows of stacked bars, one per depth level, children nested in the parent span. */
export function icicleRects(root: HierarchyNode, maxDepth = 3): IcicleRect[] {
  const rects: IcicleRect[] = [];
  const place = (nodes: HierarchyNode[], depth: number, x0: number, span: number): void => {
    if (depth >= maxDepth || nodes.length === 0 || span <= 0) return;
    const sum = nodes.reduce((acc, node) => acc + Math.max(0, node.size), 0);
    if (sum <= 0) return;
    let cursor = x0;
    for (const node of nodes) {
      const w = (Math.max(0, node.size) / sum) * span;
      rects.push({ node, depth, x: cursor, w });
      const kids = node.children ?? [];
      if (kids.length > 0) place(kids, depth + 1, cursor, w);
      cursor += w;
    }
  };
  place(root.children ?? [], 0, 0, 1);
  return rects;
}

/** One packed circle produced by {@link bubblePacking}; `x`/`y`/`r` are 0..1. */
export interface BubbleCircle {
  node: HierarchyNode;
  x: number;
  y: number;
  r: number;
}

const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

function findSpot(placed: BubbleCircle[], r: number): { x: number; y: number } | null {
  if (placed.length === 0) return { x: 0.5, y: 0.5 };
  const maxTries = 6000;
  for (let i = 1; i <= maxTries; i += 1) {
    const t = 0.016 * Math.sqrt(i);
    if (t > 1.25) break;
    const angle = i * GOLDEN_ANGLE;
    const x = 0.5 + Math.cos(angle) * t;
    const y = 0.5 + Math.sin(angle) * t;
    if (x - r < 0 || x + r > 1 || y - r < 0 || y + r > 1) continue;
    let fits = true;
    for (const other of placed) {
      const dx = x - other.x;
      const dy = y - other.y;
      if (Math.hypot(dx, dy) < r + other.r + 0.002) {
        fits = false;
        break;
      }
    }
    if (fits) return { x, y };
  }
  return null;
}

/**
 * Greedy spiral packing of `nodes` sized by `sqrt(size)`. Circles never
 * overlap and stay inside the unit square; a node that cannot be placed after
 * shrinking is dropped.
 */
export function bubblePacking(nodes: HierarchyNode[], count = 60): BubbleCircle[] {
  const items = nodes
    .filter((node) => node.size > 0)
    .slice()
    .sort((a, b) => b.size - a.size)
    .slice(0, count);
  const largest = items[0];
  if (!largest) return [];
  const total = items.reduce((acc, node) => acc + node.size, 0);
  const scale = Math.min(
    Math.sqrt(0.55 / (Math.PI * Math.max(total, 1))),
    0.42 / Math.sqrt(largest.size),
  );
  const placed: BubbleCircle[] = [];
  for (const node of items) {
    let r = Math.sqrt(node.size) * scale;
    let spot: { x: number; y: number } | null = null;
    for (let attempt = 0; attempt < 4 && spot === null; attempt += 1) {
      spot = findSpot(placed, r);
      if (spot === null) r *= 0.78;
    }
    if (spot === null) continue;
    placed.push({ node, x: spot.x, y: spot.y, r });
  }
  return placed;
}
