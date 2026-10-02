<script lang="ts">
  import type { Tile } from "@scanscan/api-types";

  import { colorForExtension, colorForKey, extensionOf, type ColorMode } from "../lib/color.js";
  import { formatBytes, percent } from "../lib/format.js";

  interface Props {
    tiles: Tile[];
    colorMode: ColorMode;
    onselect: (tile: Tile) => void;
  }

  let { tiles, colorMode, onselect }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let canvas = $state<HTMLCanvasElement | null>(null);
  let width = $state(0);
  let height = $state(0);
  let hovered = $state<Tile | null>(null);
  let pointerX = $state(0);
  let pointerY = $state(0);

  const totalSize = $derived(tiles.reduce((sum, tile) => sum + tile.size, 0));
  const hoverPercent = $derived(hovered ? percent(hovered.size, totalSize) : 0);

  function tileFill(tile: Tile): string {
    if (colorMode === "ext") {
      return colorForExtension(tile.colorKey ?? extensionOf(tile.name));
    }
    return colorForKey(tile.name);
  }

  function truncate(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
    if (maxWidth <= 0) return "";
    if (ctx.measureText(text).width <= maxWidth) return text;
    let end = text.length;
    while (end > 1 && ctx.measureText(`${text.slice(0, end)}…`).width > maxWidth) {
      end -= 1;
    }
    return `${text.slice(0, end)}…`;
  }

  function draw(): void {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const cssW = Math.max(1, width);
    const cssH = Math.max(1, height);
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);

    for (const tile of tiles) {
      const x = tile.x * cssW;
      const y = tile.y * cssH;
      const w = tile.w * cssW;
      const h = tile.h * cssH;
      if (w <= 0 || h <= 0) continue;

      ctx.fillStyle = tileFill(tile);
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = "rgba(11, 14, 20, 0.85)";
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, Math.max(0, w - 1), Math.max(0, h - 1));

      if (w >= 48 && h >= 20) {
        ctx.fillStyle = "rgba(11, 14, 20, 0.92)";
        ctx.textBaseline = "top";
        ctx.font = "600 12px ui-sans-serif, system-ui, sans-serif";
        ctx.fillText(truncate(ctx, tile.name, w - 8), x + 4, y + 4);
        ctx.font = "11px ui-sans-serif, system-ui, sans-serif";
        ctx.fillText(formatBytes(tile.size), x + 4, y + 20);
      }
    }
  }

  function hitTest(clientX: number, clientY: number): Tile | null {
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return null;
    const nx = (clientX - rect.left) / rect.width;
    const ny = (clientY - rect.top) / rect.height;
    for (let i = tiles.length - 1; i >= 0; i -= 1) {
      const tile = tiles[i];
      if (!tile) continue;
      if (nx >= tile.x && nx <= tile.x + tile.w && ny >= tile.y && ny <= tile.y + tile.h) {
        return tile;
      }
    }
    return null;
  }

  function handleClick(event: MouseEvent): void {
    const tile = hitTest(event.clientX, event.clientY);
    if (tile) onselect(tile);
  }

  function handleMove(event: MouseEvent): void {
    const tile = hitTest(event.clientX, event.clientY);
    hovered = tile;
    if (tile && canvas) {
      const rect = canvas.getBoundingClientRect();
      pointerX = event.clientX - rect.left + 12;
      pointerY = event.clientY - rect.top + 12;
    }
  }

  function handleLeave(): void {
    hovered = null;
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    const tile = hovered ?? tiles[0];
    if (!tile) return;
    event.preventDefault();
    onselect(tile);
  }

  $effect(() => {
    if (!container) return;
    const element = container;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      width = entry.contentRect.width;
      height = entry.contentRect.height;
    });
    observer.observe(element);
    return () => observer.disconnect();
  });

  $effect(() => {
    draw();
  });
</script>

<div class="treemap" bind:this={container}>
  <canvas
    bind:this={canvas}
    role="button"
    tabindex="0"
    aria-label="Disk usage treemap"
    onclick={handleClick}
    onmousemove={handleMove}
    onmouseleave={handleLeave}
    onkeydown={handleKeydown}
  ></canvas>
  {#if hovered}
    <div class="tooltip" style="left: {pointerX}px; top: {pointerY}px">
      <strong>{hovered.name}</strong>
      <span>{formatBytes(hovered.size)} · {hoverPercent.toFixed(1)}%</span>
    </div>
  {/if}
</div>

<style>
  .treemap {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }

  canvas:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .tooltip {
    position: absolute;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    max-width: 240px;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    font-size: 0.75rem;
    pointer-events: none;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
  }

  .tooltip strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip span {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
