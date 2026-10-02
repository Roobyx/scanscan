<script lang="ts">
  import type { HierarchyNode } from "@scanscan/api-types";

  import { colorForKey } from "../lib/color.js";
  import { formatBytes } from "../lib/format.js";
  import { bubblePacking } from "../lib/layout.js";

  interface Props {
    root: HierarchyNode;
    onselect: (node: HierarchyNode) => void;
  }

  let { root, onselect }: Props = $props();

  const VIEW = 1000;
  const circles = $derived(bubblePacking(root.children ?? [], 60));
  let hovered = $state<{ node: HierarchyNode; x: number; y: number } | null>(null);

  function handleKeydown(event: KeyboardEvent, node: HierarchyNode): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    onselect(node);
  }

  function handleMove(event: MouseEvent, node: HierarchyNode): void {
    hovered = { node, x: event.clientX, y: event.clientY };
  }
</script>

<div class="bubble">
  <svg viewBox="0 0 {VIEW} {VIEW}" role="group" aria-label="Bubble chart of disk usage">
    {#each circles as circle (circle.node.id)}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions a11y_no_noninteractive_tabindex -->
      <g
        role="button"
        tabindex="0"
        aria-label={`${circle.node.name}, ${formatBytes(circle.node.size)}`}
        onclick={() => onselect(circle.node)}
        onkeydown={(event) => handleKeydown(event, circle.node)}
        onmousemove={(event) => handleMove(event, circle.node)}
        onmouseleave={() => (hovered = null)}
      >
        <circle
          class="disc"
          cx={circle.x * VIEW}
          cy={circle.y * VIEW}
          r={circle.r * VIEW}
          fill={colorForKey(circle.node.name)}
          stroke="var(--bg)"
          stroke-width="1.5"
        />
        {#if circle.r * VIEW > 26}
          <text
            class="label"
            x={circle.x * VIEW}
            y={circle.y * VIEW}
            text-anchor="middle"
            dominant-baseline="middle"
          >
            {circle.node.name}
          </text>
        {/if}
      </g>
    {/each}
  </svg>
  {#if hovered}
    <div class="tooltip" style="left: {hovered.x + 14}px; top: {hovered.y + 14}px">
      <strong>{hovered.node.name}</strong>
      <span>{formatBytes(hovered.node.size)}</span>
    </div>
  {/if}
</div>

<style>
  .bubble {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  g {
    cursor: pointer;
  }

  g:hover .disc {
    opacity: 0.86;
  }

  g:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .label {
    fill: rgba(11, 14, 20, 0.92);
    font-size: 20px;
    font-weight: 600;
    pointer-events: none;
  }

  .tooltip {
    position: fixed;
    z-index: 5;
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
