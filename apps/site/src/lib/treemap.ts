export type Rect = { x: number; y: number; w: number; h: number };

export type TreemapItem<T> = { value: number; data: T };

export type TreemapTile<T> = Rect & { data: T };

type Area<T> = TreemapItem<T> & { area: number };

function worstRatio<T>(row: Area<T>[], side: number): number {
  if (row.length === 0) return Number.POSITIVE_INFINITY;
  let sum = 0;
  let max = 0;
  let min = Number.POSITIVE_INFINITY;
  for (const item of row) {
    sum += item.area;
    max = Math.max(max, item.area);
    min = Math.min(min, item.area);
  }
  if (sum <= 0 || min <= 0 || side <= 0) return Number.POSITIVE_INFINITY;
  const side2 = side * side;
  return Math.max((side2 * max) / (sum * sum), (sum * sum) / (side2 * min));
}

/**
 * Squarified treemap (Bruls, Huizing, van Wijk). Values are laid out in a rect
 * so that area is proportional to value and tiles stay as square as possible.
 */
export function squarify<T>(items: TreemapItem<T>[], rect: Rect): TreemapTile<T>[] {
  const total = items.reduce((sum, item) => sum + Math.max(0, item.value), 0);
  if (total <= 0 || rect.w <= 0 || rect.h <= 0) return [];

  const scale = (rect.w * rect.h) / total;
  const areas: Area<T>[] = items
    .map((item) => ({ ...item, area: Math.max(0, item.value) * scale }))
    .filter((item) => item.area > 0)
    .sort((a, b) => b.area - a.area);

  const tiles: TreemapTile<T>[] = [];
  let x = rect.x;
  let y = rect.y;
  let w = rect.w;
  let h = rect.h;
  let row: Area<T>[] = [];
  let index = 0;

  const flush = () => {
    if (row.length === 0) return;
    const sum = row.reduce((acc, item) => acc + item.area, 0);
    if (sum <= 0) {
      row = [];
      return;
    }
    if (w >= h) {
      const rw = Math.min(w, sum / h);
      let ry = y;
      for (const item of row) {
        const rh = rw > 0 ? Math.min(h - (ry - y), item.area / rw) : 0;
        tiles.push({ x, y: ry, w: rw, h: Math.max(0, rh), data: item.data });
        ry += rh;
      }
      x += rw;
      w = Math.max(0, w - rw);
    } else {
      const rh = Math.min(h, sum / w);
      let rx = x;
      for (const item of row) {
        const rw = rh > 0 ? Math.min(w - (rx - x), item.area / rh) : 0;
        tiles.push({ x: rx, y, w: Math.max(0, rw), h: rh, data: item.data });
        rx += rw;
      }
      y += rh;
      h = Math.max(0, h - rh);
    }
    row = [];
  };

  while (index < areas.length) {
    const next = areas[index];
    if (!next) break;
    const side = Math.min(w, h);
    if (row.length === 0 || worstRatio([...row, next], side) <= worstRatio(row, side)) {
      row.push(next);
      index += 1;
    } else {
      flush();
    }
    if (w <= 0.001 || h <= 0.001) break;
  }
  flush();
  return tiles;
}
