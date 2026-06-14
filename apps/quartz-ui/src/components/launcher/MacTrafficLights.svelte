<script lang="ts">
  import { closeWindow, minimizeWindow, maximizeWindow } from '$lib/api';

  async function safe(fn: () => Promise<void>) {
    try {
      await fn();
    } catch {

    }
  }
</script>

<div class="traffic-lights">
  <button
    class="dot dot-close"
    type="button"
    aria-label="Close"
    onclick={() => safe(closeWindow)}
  ></button>
  <button
    class="dot dot-minimize"
    type="button"
    aria-label="Minimize"
    onclick={() => safe(minimizeWindow)}
  ></button>
  <button
    class="dot dot-maximize"
    type="button"
    aria-label="Maximize"
    onclick={() => safe(maximizeWindow)}
  ></button>
</div>

<style>
  .traffic-lights {
    display: flex;
    align-items: center;
    gap: 8px;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: none;
    padding: 0;
    cursor: default;
    transition: filter var(--duration-fast) var(--ease-smooth);
  }

  .dot-close {
    background: #ff5f57;
  }

  .dot-minimize {
    background: #febc2e;
  }

  .dot-maximize {
    background: #28c840;
  }

  .traffic-lights:hover .dot-close {
    filter: brightness(0.92);
  }

  .traffic-lights:hover .dot-minimize {
    filter: brightness(0.92);
  }

  .traffic-lights:hover .dot-maximize {
    filter: brightness(0.92);
  }
</style>
