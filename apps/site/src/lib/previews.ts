import { KIND_META, type SampleNode } from "../data/sample.js";

export type SunburstArc = {
  node: SampleNode;
  depth: number;
  path: string;
  color: string;
};

function polar(cx: number, cy: number, radius: number, angle: number): [number, number] {
  return [cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius];
}

function annulusPath(
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

export function sunburstArcs(root: SampleNode, size = 400, rings = 2): SunburstArc[] {
  const cx = size / 2;
  const cy = size / 2;
  const outer = size / 2 - 8;
  const inner = outer * 0.3;
  const thickness = (outer - inner) / rings;
  const arcs: SunburstArc[] = [];
  const gap = 0.008;

  const place = (nodes: SampleNode[], depth: number, from: number, extent: number, r0: number) => {
    if (depth >= rings || nodes.length === 0 || extent <= 0) return;
    const sum = nodes.reduce((acc, node) => acc + Math.max(0, node.size), 0);
    if (sum <= 0) return;
    const r1 = Math.min(outer, r0 + thickness);
    let angle = from;
    for (const node of nodes) {
      const span = (Math.max(0, node.size) / sum) * extent;
      const a0 = angle + gap / 2;
      const a1 = angle + span - gap / 2;
      angle += span;
      if (a1 <= a0) continue;
      arcs.push({
        node,
        depth: depth + 1,
        path: annulusPath(cx, cy, r0, r1, a0, a1),
        color: KIND_META[node.kind].color,
      });
      if (node.children) place(node.children, depth + 1, a0, a1 - a0, r1);
    }
  };

  place(root.children ?? [], 0, -Math.PI / 2, Math.PI * 2, inner);
  return arcs;
}

export type IcicleRect = { node: SampleNode; depth: number; x: number; w: number };

export function icicleRects(root: SampleNode, maxDepth = 3): IcicleRect[] {
  const rects: IcicleRect[] = [];
  const place = (nodes: SampleNode[], depth: number, x0: number, span: number) => {
    if (depth >= maxDepth || nodes.length === 0 || span <= 0) return;
    const sum = nodes.reduce((acc, node) => acc + Math.max(0, node.size), 0);
    if (sum <= 0) return;
    let cursor = x0;
    for (const node of nodes) {
      const w = (Math.max(0, node.size) / sum) * span;
      rects.push({ node, depth, x: cursor, w });
      if (node.children) place(node.children, depth + 1, cursor, w);
      cursor += w;
    }
  };
  place(root.children ?? [], 0, 0, 1);
  return rects;
}

export type Bubble = { node: SampleNode; x: number; y: number; r: number };

const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

function findSpot(placed: Bubble[], r: number): { x: number; y: number } | null {
  if (placed.length === 0) return { x: 0.5, y: 0.5 };
  for (let i = 1; i <= 6000; i += 1) {
    const t = 0.018 * Math.sqrt(i);
    if (t > 1.2) break;
    const angle = i * GOLDEN_ANGLE;
    const x = 0.5 + Math.cos(angle) * t;
    const y = 0.5 + Math.sin(angle) * t;
    if (x - r < 0 || x + r > 1 || y - r < 0 || y + r > 1) continue;
    let fits = true;
    for (const other of placed) {
      if (Math.hypot(x - other.x, y - other.y) < r + other.r + 0.003) {
        fits = false;
        break;
      }
    }
    if (fits) return { x, y };
  }
  return null;
}

export function bubblePacking(nodes: SampleNode[], count = 8): Bubble[] {
  const items = nodes
    .filter((node) => node.size > 0)
    .slice()
    .sort((a, b) => b.size - a.size)
    .slice(0, count);
  const largest = items[0];
  if (!largest) return [];
  const total = items.reduce((sum, node) => sum + node.size, 0);
  const scale = Math.min(
    Math.sqrt(0.5 / (Math.PI * Math.max(total, 1))),
    0.4 / Math.sqrt(largest.size),
  );
  const placed: Bubble[] = [];
  for (const node of items) {
    let r = Math.sqrt(node.size) * scale;
    let spot: { x: number; y: number } | null = null;
    for (let attempt = 0; attempt < 5 && spot === null; attempt += 1) {
      spot = findSpot(placed, r);
      if (spot === null) r *= 0.8;
    }
    if (spot === null) continue;
    placed.push({ node, x: spot.x, y: spot.y, r });
  }
  return placed;
}
