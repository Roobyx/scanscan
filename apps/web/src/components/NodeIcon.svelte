<script lang="ts">
  import type { NodeKind } from "@scanscan/api-types";

  interface Props {
    kind: NodeKind;
    /** Optional tint, e.g. an extension colour for a file. */
    color?: string;
  }

  let { kind, color }: Props = $props();

  const tint = $derived.by(() => color ?? "var(--muted)");
</script>

<span class="icon" role="img" aria-label={kind}>
  {#if kind === "directory"}
    <svg viewBox="0 0 16 16">
      <path d="M1.4 4.2 h4.4 l1.5 1.7 H14.6 V12.7 H1.4 Z" fill={tint} stroke-width="1" />
    </svg>
  {:else if kind === "symlink"}
    <svg viewBox="0 0 16 16">
      <path d="M4 1.6 h5.4 L12.6 5.1 V14.4 H4 Z" fill={tint} stroke-width="1" />
      <path
        d="M6.2 8.6 h4.1 M8.7 7 l1.6 1.6 l-1.6 1.6"
        fill="none"
        stroke="var(--text)"
        stroke-width="1.3"
      />
    </svg>
  {:else if kind === "special"}
    <svg viewBox="0 0 16 16">
      <path d="M8 1.4 L14.6 8 L8 14.6 L1.4 8 Z" fill={tint} stroke-width="1" />
    </svg>
  {:else}
    <svg viewBox="0 0 16 16">
      <path d="M4 1.6 h5.4 L12.6 5.1 V14.4 H4 Z" fill={tint} stroke-width="1" />
      <path d="M9.4 1.6 V5.1 H12.6" fill="none" stroke="var(--bg)" stroke-width="1.2" />
    </svg>
  {/if}
</span>

<style>
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
  }

  svg {
    display: block;
    width: 14px;
    height: 14px;
  }
</style>
