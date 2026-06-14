<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    subtitle?: string;
    trailing?: 'chevron' | 'check' | 'none';
    active?: boolean;
    onclick?: () => void;
    disabled?: boolean;
    icon?: Snippet;
    accessory?: Snippet;
  }

  let {
    label,
    subtitle,
    trailing = 'none',
    active = false,
    onclick,
    disabled = false,
    icon,
    accessory,
  }: Props = $props();

  const Tag = onclick ? 'button' : 'div';
</script>

<svelte:element
  this={Tag}
  class="glass-row"
  class:active
  class:clickable={!!onclick}
  {disabled}
  onclick={onclick}
  type={onclick ? 'button' : undefined}
>
  {#if icon}
    <span class="row-icon">{@render icon()}</span>
  {/if}

  <span class="row-body">
    <span class="row-label">{label}</span>
    {#if subtitle}
      <span class="row-sub">{subtitle}</span>
    {/if}
  </span>

  {#if accessory}
    <span class="row-accessory">{@render accessory()}</span>
  {:else if trailing === 'chevron'}
    <span class="row-chevron" aria-hidden="true">›</span>
  {:else if trailing === 'check'}
    <span class="row-check" aria-hidden="true">✓</span>
  {/if}
</svelte:element>

<style>
  .glass-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 11px 14px;
    text-align: left;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    transition: background var(--duration-fast) var(--ease-smooth);
  }

  .glass-row + .glass-row {
    border-top: 1px solid var(--group-divider);
  }

  .glass-row.clickable:not(:disabled):hover {
    background: var(--row-hover);
  }

  .glass-row.active {
    background: var(--row-active);
  }

  .glass-row:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .row-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    overflow: hidden;
  }

  .row-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .row-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .row-sub {
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-accessory {
    flex-shrink: 0;
  }

  .row-chevron {
    flex-shrink: 0;
    font-size: 18px;
    font-weight: 300;
    color: var(--text-tertiary);
    line-height: 1;
  }

  .row-check {
    flex-shrink: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--accent);
  }
</style>
