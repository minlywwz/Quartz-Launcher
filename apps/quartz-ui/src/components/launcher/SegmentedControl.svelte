<script lang="ts">
  interface Option {
    id: string;
    label: string;
  }

  interface Props {
    options: Option[];
    value: string;
    onchange?: (id: string) => void;
    ariaLabel?: string;
  }

  let { options, value, onchange, ariaLabel = 'Segmented control' }: Props = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let thumbStyle = $state('');

  $effect(() => {
    if (!trackEl) return;
    value;
    options;
    const run = () => requestAnimationFrame(updateThumb);
    run();
    const ro = new ResizeObserver(run);
    ro.observe(trackEl);
    return () => ro.disconnect();
  });

  function updateThumb() {
    if (!trackEl) return;
    const idx = options.findIndex((o) => o.id === value);
    if (idx < 0) return;
    const buttons = trackEl.querySelectorAll<HTMLButtonElement>('.seg-btn');
    const btn = buttons[idx];
    if (!btn) return;
    thumbStyle = `transform: translateX(${btn.offsetLeft}px); width: ${btn.offsetWidth}px;`;
  }
</script>

<div class="segmented" role="tablist" aria-label={ariaLabel} bind:this={trackEl}>
  <div class="seg-thumb" style={thumbStyle} aria-hidden="true"></div>
  {#each options as opt (opt.id)}
    <button
      class="seg-btn"
      class:active={value === opt.id}
      type="button"
      role="tab"
      aria-selected={value === opt.id}
      onclick={() => onchange?.(opt.id)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: flex;
    align-items: stretch;
    padding: 3px;
    border-radius: var(--control-radius-pill);
    background: var(--seg-track-bg);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.06);
    gap: 0;
    min-height: 36px;
  }

  .seg-thumb {
    position: absolute;
    top: 3px;
    left: 0;
    height: calc(100% - 6px);
    border-radius: var(--control-radius-pill);
    background: var(--seg-thumb-bg);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.08),
      0 0 0 0.5px rgba(0, 0, 0, 0.04);
    transition:
      transform var(--duration-panel) var(--ease-ios),
      width var(--duration-panel) var(--ease-ios);
    pointer-events: none;
    z-index: 0;
  }

  .seg-btn {
    position: relative;
    z-index: 1;
    flex: 1;
    padding: 7px 14px;
    border-radius: var(--control-radius-pill);
    font-size: 13px;
    font-weight: 600;
    color: var(--seg-label);
    white-space: nowrap;
    transition: color var(--duration-panel) var(--ease-ios);
  }

  .seg-btn.active {
    color: var(--seg-label-active);
  }
</style>
