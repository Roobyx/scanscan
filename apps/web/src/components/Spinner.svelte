<script lang="ts">
  interface Props {
    /** Diameter in CSS pixels. */
    size?: number;
    /** Human-readable text for assistive tech. */
    label?: string;
  }

  let { size = 16, label = "Loading" }: Props = $props();

  const stroke = $derived.by(() => Math.max(2, Math.round(size / 8)));
</script>

<span
  class="spinner"
  role="status"
  aria-label={label}
  style="width: {size}px; height: {size}px; border-width: {stroke}px"
></span>

<style>
  .spinner {
    display: inline-block;
    flex: none;
    border-style: solid;
    border-color: color-mix(in srgb, var(--accent) 22%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spinn 0.7s linear infinite;
  }

  @keyframes spinn {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
