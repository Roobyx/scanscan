<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon from "./Icon.svelte";

  interface Props {
    title: string;
    subtitle?: string | null;
    onclose: () => void;
    children: Snippet;
    wide?: boolean;
  }

  let { title, subtitle = null, onclose, children, wide = false }: Props = $props();

  let dialog = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = previous;
    };
  });

  $effect(() => {
    dialog?.focus();
  });

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onclose();
    }
  }

  function handleBackdrop(event: MouseEvent): void {
    if (event.target === event.currentTarget) onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="backdrop" role="presentation" onclick={handleBackdrop}>
  <div
    class="dialog"
    class:wide={wide}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    bind:this={dialog}
  >
    <header>
      <div class="heading">
        <h2>{title}</h2>
        {#if subtitle}
          <p>{subtitle}</p>
        {/if}
      </div>
      <button type="button" class="close" onclick={onclose} aria-label="Close dialog">
        <Icon name="x-circle" size={18} />
      </button>
    </header>
    <div class="body">
      {@render children()}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 1rem;
    background: color-mix(in srgb, #05070b 68%, transparent);
    backdrop-filter: blur(3px);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    width: min(620px, 94vw);
    max-height: min(88vh, 760px);
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--panel);
    box-shadow:
      0 24px 70px rgba(0, 0, 0, 0.55),
      0 0 0 1px color-mix(in srgb, var(--accent) 10%, transparent);
    animation: enter 140ms ease-out;
  }

  .dialog.wide {
    width: min(980px, 96vw);
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem 1.15rem;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--panel) 88%, var(--accent) 12%),
      var(--panel)
    );
  }

  .heading {
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: 1.05rem;
    letter-spacing: 0.01em;
  }

  .heading p {
    margin: 0.25rem 0 0;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .close {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid transparent;
    border-radius: 8px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }

  .close:hover {
    border-color: var(--border);
    background: var(--panel-2);
    color: var(--text);
  }

  .body {
    min-height: 0;
    padding: 1.1rem 1.15rem 1.25rem;
    overflow: auto;
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (max-width: 640px) {
    .backdrop {
      padding: 0;
    }

    .dialog,
    .dialog.wide {
      width: 100vw;
      max-height: 100vh;
      border-radius: 0;
    }
  }
</style>
