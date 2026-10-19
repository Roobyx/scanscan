/**
 * Tooltip placement beside the pointer.
 *
 * A tooltip is anchored to the same box it lives in (a chart host), so it is cut
 * off when the pointer is close to the right or bottom edge. Placing it by the
 * opposite edge instead keeps it inside that box without measuring it: the
 * anchor is the tooltip's own right/bottom edge, so it grows back towards the
 * pointer.
 */

export interface TooltipBox {
  /** Pointer position, relative to the box the tooltip is anchored in. */
  x: number;
  y: number;
  /** Size of that box. */
  width: number;
  height: number;
  /** Space kept between the pointer and the tooltip. */
  gap?: number;
  /** Room the tooltip needs beside the pointer before it flips to the other side. */
  needWidth?: number;
  needHeight?: number;
}

const GAP = 14;
const NEED_WIDTH = 240;
const NEED_HEIGHT = 92;

/** A `style` value placing a tooltip beside the pointer, flipped at the edges. */
export function tooltipStyle(box: TooltipBox): string {
  const gap = box.gap ?? GAP;
  const needW = box.needWidth ?? NEED_WIDTH;
  const needH = box.needHeight ?? NEED_HEIGHT;

  const roomLeft = box.x - gap;
  const roomRight = box.width - box.x - gap;
  const roomTop = box.y - gap;
  const roomBottom = box.height - box.y - gap;

  // Flip only when the default side has no room and the other side has more;
  // with the pointer outside the box both rooms are negative and it stays put.
  const flipsX = roomRight < needW && roomLeft > roomRight;
  const flipsY = roomBottom < needH && roomTop > roomBottom;

  const across = flipsX
    ? `right: ${Math.max(0, box.width - box.x + gap)}px`
    : `left: ${Math.max(0, box.x + gap)}px`;
  const down = flipsY
    ? `bottom: ${Math.max(0, box.height - box.y + gap)}px`
    : `top: ${Math.max(0, box.y + gap)}px`;
  return `${across}; ${down}`;
}
