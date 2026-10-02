import type { HierarchyNode } from "@scanscan/api-types";
import { describe, expect, test } from "vitest";

import { bubblePacking, icicleRects, sunburstArcs } from "./layout.js";

function node(id: number, size: number, children?: HierarchyNode[]): HierarchyNode {
  return { id, name: `n${id}`, kind: children ? "directory" : "file", size, children };
}

const TREE: HierarchyNode = node(0, 100, [
  node(1, 50, [node(3, 30), node(4, 20)]),
  node(2, 50, [node(5, 10), node(6, 40)]),
]);

describe("sunburstArcs", () => {
  test("emits one arc per descendant up to the ring limit", () => {
    const arcs = sunburstArcs(TREE, { rings: 2 });
    expect(arcs.map((arc) => arc.node.id).sort((a, b) => a - b)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(arcs.every((arc) => arc.path.startsWith("M "))).toBe(true);
    expect(arcs.every((arc) => arc.depth >= 1 && arc.depth <= 2)).toBe(true);
  });

  test("the first ring spans a full turn proportional to size", () => {
    const arcs = sunburstArcs(TREE, { rings: 1 });
    const total = arcs.reduce((acc, arc) => acc + arc.angle, 0);
    expect(total).toBeCloseTo(Math.PI * 2, 6);
    const first = arcs.find((arc) => arc.node.id === 1);
    const second = arcs.find((arc) => arc.node.id === 2);
    expect(first?.angle).toBeCloseTo(second?.angle ?? -1, 6);
  });
});

describe("icicleRects", () => {
  test("children tile the parent span without gaps or overlap", () => {
    const rects = icicleRects(TREE, 1);
    const top = rects.filter((rect) => rect.depth === 0).sort((a, b) => a.x - b.x);
    expect(top).toHaveLength(2);
    expect(top[0]?.x).toBeCloseTo(0, 9);
    const last = top[top.length - 1];
    expect((last?.x ?? 0) + (last?.w ?? 0)).toBeCloseTo(1, 9);
  });

  test("nested cells stay inside their parent span", () => {
    const rects = icicleRects(TREE, 2);
    const parent = rects.find((rect) => rect.node.id === 1);
    const child = rects.find((rect) => rect.node.id === 3);
    expect(parent).toBeDefined();
    expect(child).toBeDefined();
    if (parent && child) {
      expect(child.x).toBeGreaterThanOrEqual(parent.x);
      expect(child.x + child.w).toBeLessThanOrEqual(parent.x + parent.w + 1e-9);
    }
  });
});

describe("bubblePacking", () => {
  test("packs circles inside the unit square without overlap", () => {
    const circles = bubblePacking(TREE.children ?? []);
    expect(circles).toHaveLength(2);
    for (const circle of circles) {
      expect(circle.x - circle.r).toBeGreaterThanOrEqual(-1e-9);
      expect(circle.x + circle.r).toBeLessThanOrEqual(1 + 1e-9);
      expect(circle.y - circle.r).toBeGreaterThanOrEqual(-1e-9);
      expect(circle.y + circle.r).toBeLessThanOrEqual(1 + 1e-9);
    }
    const [a, b] = circles;
    if (a && b) {
      const distance = Math.hypot(a.x - b.x, a.y - b.y);
      expect(distance).toBeGreaterThanOrEqual(a.r + b.r);
    }
  });

  test("returns nothing for an empty input", () => {
    expect(bubblePacking([])).toEqual([]);
  });
});
