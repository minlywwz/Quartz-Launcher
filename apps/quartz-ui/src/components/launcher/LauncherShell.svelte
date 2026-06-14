<script lang="ts">
  import { launcherPanel, type LauncherPanel } from '$lib/stores/launcher';
  import { blurSlideIn, blurSlideOut, panelDirection, PANEL_TRANSITION_MS } from '$lib/transitions/panel';
  import MacTrafficLights from './MacTrafficLights.svelte';
  import LauncherHome from './LauncherHome.svelte';
  import BrowseView from './BrowseView.svelte';
  import LauncherSettings from './LauncherSettings.svelte';
  import BottomBar from './BottomBar.svelte';
  import type { InstanceInfo } from '$lib/api';

  interface Props {
    instances: InstanceInfo[];
    selectedInstance: InstanceInfo | null;
    onselectinstance?: (instance: InstanceInfo) => void;
    oncreated?: (instance: InstanceInfo) => void;
    instancesReady?: boolean;
  }

  let {
    instances,
    selectedInstance,
    onselectinstance,
    oncreated,
    instancesReady = true,
  }: Props = $props();

  const panel = $derived($launcherPanel);
  const bottomBarVisible = $derived(panel !== 'browse');

  let previousPanel: LauncherPanel = 'home';
  let direction = $state(1);

  $effect.pre(() => {
    const next = $launcherPanel;
    if (next !== previousPanel) {
      direction = panelDirection(previousPanel, next);
      previousPanel = next;
    }
  });
</script>

<div class="launcher-shell" style="--panel-transition: {PANEL_TRANSITION_MS}ms">
  <header class="launcher-titlebar" data-tauri-drag-region>
    <div class="titlebar-left">
      <MacTrafficLights />
    </div>
    <span class="titlebar-title" data-tauri-drag-region>Minecraft Launcher</span>
    <div class="titlebar-right" aria-hidden="true"></div>
  </header>

  <main class="launcher-main">
    {#key panel}
      <div
        class="panel-layer"
        in:blurSlideIn={{ direction }}
        out:blurSlideOut={{ direction }}
      >
        {#if panel === 'home'}
          <LauncherHome instance={selectedInstance} ready={instancesReady} />
        {:else if panel === 'browse'}
          <BrowseView oncreated={oncreated} />
        {:else if panel === 'settings'}
          <LauncherSettings />
        {/if}
      </div>
    {/key}
  </main>

  <div class="bottom-slot" class:bottom-slot--hidden={!bottomBarVisible}>
    <div class="bottom-slot-inner">
      <BottomBar {instances} {selectedInstance} onselect={onselectinstance} />
    </div>
  </div>
</div>

<style>
  .launcher-shell {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--launcher-bg);
    overflow: hidden;
  }

  .launcher-titlebar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    height: 40px;
    padding: 0 14px;
    flex-shrink: 0;
    -webkit-app-region: drag;
    app-region: drag;
    border-bottom: 1px solid var(--glass-border-subtle);
  }

  .titlebar-left {
    justify-self: start;
    -webkit-app-region: no-drag;
    app-region: no-drag;
  }

  .titlebar-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-tertiary);
    letter-spacing: 0.01em;
    -webkit-app-region: drag;
    app-region: drag;
  }

  .titlebar-right {
    justify-self: end;
  }

  .launcher-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .panel-layer {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--launcher-bg);
    will-change: opacity, transform, filter;
    backface-visibility: hidden;
  }

  .bottom-slot {
    display: grid;
    grid-template-rows: 1fr;
    flex-shrink: 0;
    position: relative;
    z-index: 1000;
    opacity: 1;
    transition:
      grid-template-rows var(--panel-transition) cubic-bezier(0.4, 0, 0.15, 1),
      opacity var(--panel-transition) cubic-bezier(0.4, 0, 0.15, 1);
  }

  .bottom-slot--hidden {
    grid-template-rows: 0fr;
    opacity: 0;
    pointer-events: none;
  }

  .bottom-slot-inner {
    min-height: 0;
    overflow: visible;
  }

  .bottom-slot--hidden .bottom-slot-inner {
    overflow: hidden;
  }

  @media (prefers-reduced-motion: reduce) {
    .bottom-slot {
      transition: none;
    }

    .panel-layer {
      will-change: auto;
    }
  }
</style>
