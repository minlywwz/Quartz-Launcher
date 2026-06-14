<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label?: string;
    footer?: string;
    children: Snippet;
  }

  let { label, footer, children }: Props = $props();
</script>

<section class="glass-group">
  {#if label}
    <h2 class="group-label">{label}</h2>
  {/if}
  <div class="group-panel liquid-panel">
    {@render children()}
  </div>
  {#if footer}
    <p class="group-footer">{footer}</p>
  {/if}
</section>

<style>
  .glass-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .group-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    padding: 0 4px;
  }

  .group-panel {
    border-radius: var(--group-radius);
  }

  .group-panel :global(.glass-row:first-child) {
    border-top-left-radius: calc(var(--group-radius) - 1px);
    border-top-right-radius: calc(var(--group-radius) - 1px);
  }

  .group-panel :global(.glass-row:last-child) {
    border-bottom-left-radius: calc(var(--group-radius) - 1px);
    border-bottom-right-radius: calc(var(--group-radius) - 1px);
  }

  .group-footer {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-tertiary);
    padding: 0 4px;
  }
</style>
