<script lang="ts">
  interface Crumb {
    id: number;
    name: string;
  }

  interface Props {
    path: Crumb[];
    onnavigate: (index: number) => void;
  }

  let { path, onnavigate }: Props = $props();
</script>

<nav aria-label="Breadcrumb">
  {#each path as crumb, index (crumb.id)}
    {#if index > 0}<span class="sep">/</span>{/if}
    {#if index === path.length - 1}
      <span class="current" title={crumb.name}>{crumb.name}</span>
    {:else}
      <button type="button" onclick={() => onnavigate(index)} title={crumb.name}>{crumb.name}</button>
    {/if}
  {/each}
</nav>

<style>
  nav {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    overflow: hidden;
    font-size: 0.8rem;
  }

  button {
    max-width: 16rem;
    padding: 0.1rem 0.25rem;
    overflow: hidden;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--accent);
    font: inherit;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  button:hover {
    background: var(--panel-2);
  }

  .current {
    max-width: 20rem;
    overflow: hidden;
    color: var(--text);
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sep {
    color: var(--muted);
  }
</style>
