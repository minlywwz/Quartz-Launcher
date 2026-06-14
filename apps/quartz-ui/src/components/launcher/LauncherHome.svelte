<script lang="ts">
  import type { InstanceInfo, InstanceRunState } from '$lib/api';
  import {
    launchInstance,
    getInstanceRunState,
    stopInstanceGame,
    formatInvokeError,
  } from '$lib/api';
  import { settings } from '$lib/stores/settings';
  import { launcherPanel } from '$lib/stores/launcher';
  import { Play, Plus } from '$lib/icons';
  import InstanceHeroIcon from './InstanceHeroIcon.svelte';

  interface Props {
    instance: InstanceInfo | null;
    ready: boolean;
    statusText?: string;
  }

  let { instance, ready, statusText = '' }: Props = $props();

  let launching = $state(false);
  let stopping = $state(false);
  let launchStatus = $state('');
  let runState = $state<InstanceRunState | null>(null);

  const isRunning = $derived(Boolean(runState?.running));
  const sessionCount = $derived(runState?.sessionCount ?? 0);

  const displayStatus = $derived(
    launching
      ? 'Launching…'
      : stopping
        ? 'Closing game…'
        : launchStatus || statusText || (instance ? '' : 'Select an instance')
  );

  async function refreshRunState(instanceId: string) {
    try {
      runState = await getInstanceRunState(instanceId);
    } catch {

    }
  }

  $effect(() => {
    const id = instance?.id;
    if (!id) {
      runState = null;
      return;
    }

    void refreshRunState(id);
    const timer = setInterval(() => {
      void refreshRunState(id);
    }, 2000);

    return () => clearInterval(timer);
  });

  async function handlePlay() {
    if (!instance || launching) return;
    launching = true;
    launchStatus = '';
    try {
      await settings.flush();
      const result = await launchInstance(instance.id);
      launchStatus = result.success
        ? (result.message ?? 'Game launched')
        : (result.message ?? 'Launch failed');
      if (result.success) {
        await refreshRunState(instance.id);
      }
    } catch (e) {
      launchStatus = formatInvokeError(e);
    } finally {
      launching = false;
    }
  }

  async function handleStop() {
    if (!instance || stopping || !isRunning) return;
    stopping = true;
    launchStatus = '';
    try {
      const stopped = await stopInstanceGame(instance.id);
      launchStatus =
        stopped > 0 ? 'Game closed' : 'No running game found';
      await refreshRunState(instance.id);
    } catch (e) {
      launchStatus = formatInvokeError(e);
    } finally {
      stopping = false;
    }
  }

  function openBrowse() {
    launcherPanel.goBrowse();
  }
</script>

<div class="home">
  <div class="home-center">
    <div class="hero-wrap">
      <InstanceHeroIcon
        iconUrl={instance?.iconUrl}
        instanceId={instance?.id}
        name={instance?.name ?? 'Minecraft'}
        size={120}
      />
    </div>

    <h1 class="title">{instance?.name ?? 'Minecraft'}</h1>

    {#if isRunning}
      <div class="running-badge" aria-live="polite">
        <span class="running-dot" aria-hidden="true"></span>
        <span>Minecraft is running</span>
        {#if sessionCount > 1}
          <span class="running-count">×{sessionCount}</span>
        {/if}
      </div>
    {/if}

    {#if displayStatus}
      <p class="status">{displayStatus}</p>
    {/if}

    <div class="actions">
      <button
        class="play-btn"
        type="button"
        disabled={!instance || launching || stopping}
        onclick={handlePlay}
      >
        <span>{launching ? 'Launching…' : isRunning ? 'Launch another' : 'Play'}</span>
        <span class="play-icon" aria-hidden="true">
          <Play size={14} />
        </span>
      </button>

      {#if isRunning}
        <div class="game-control-group">
          <button
            class="stop-btn"
            type="button"
            aria-label="Close Minecraft"
            disabled={stopping || launching}
            onclick={handleStop}
          >
            <span aria-hidden="true">×</span>
          </button>
          <button
            class="launch-more-btn"
            type="button"
            aria-label="Launch another Minecraft window"
            disabled={launching || stopping}
            onclick={handlePlay}
          >
            <Plus size={18} />
          </button>
        </div>
      {/if}

      <button class="add-btn" type="button" aria-label="Add instance" onclick={openBrowse}>
        <Plus size={22} />
      </button>
    </div>

    {#if !instance && ready}
      <p class="hint">Tap + to create an instance or install a modpack from Modrinth</p>
    {/if}

    {#if !$settings.microsoftLinked && $settings.offlineUsername}
      <p class="offline-note">Offline mode: online-mode servers require a Microsoft account</p>
    {/if}
  </div>
</div>

<style>
  .home {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 12px 24px 8px;
    min-height: 0;
  }

  .home-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    width: 100%;
    max-width: 360px;
  }

  .hero-wrap {
    margin-bottom: 20px;
    min-height: 130px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .title {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.03em;
    color: var(--text-primary);
    margin-bottom: 12px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .running-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    padding: 6px 12px;
    border-radius: var(--control-radius-pill);
    background: rgba(76, 175, 80, 0.12);
    color: #2e7d32;
    font-size: 13px;
    font-weight: 600;
  }

  .running-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #43a047;
    box-shadow: 0 0 0 3px rgba(67, 160, 71, 0.22);
    animation: pulse 1.8s ease-in-out infinite;
  }

  .running-count {
    opacity: 0.85;
    font-variant-numeric: tabular-nums;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(0.92);
      opacity: 0.75;
    }
  }

  .status {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    margin: 0 0 20px;
  }

  .actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .play-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    max-width: 320px;
    padding: 16px 24px;
    border-radius: var(--control-radius-pill);
    background: var(--play-btn-bg);
    color: var(--text-primary);
    font-size: 17px;
    font-weight: 600;
    box-shadow:
      0 4px 14px rgba(0, 0, 0, 0.08),
      inset 0 1px 0 rgba(255, 255, 255, 0.85);
    transition:
      transform 0.2s cubic-bezier(0.32, 0.72, 0, 1),
      box-shadow 0.2s var(--ease-smooth),
      opacity 0.2s var(--ease-smooth);
  }

  .play-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.1),
      inset 0 1px 0 rgba(255, 255, 255, 0.9);
  }

  .play-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .play-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .play-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--control-bg);
    color: var(--text-secondary);
  }

  .game-control-group {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    position: relative;
  }

  .stop-btn,
  .launch-more-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--control-bg);
    color: var(--text-secondary);
    transition:
      transform 0.18s var(--ease-smooth),
      opacity 0.18s var(--ease-smooth),
      width 0.18s var(--ease-smooth),
      margin 0.18s var(--ease-smooth),
      background 0.18s var(--ease-smooth),
      color 0.18s var(--ease-smooth);
  }

  .stop-btn {
    font-size: 26px;
    line-height: 1;
    font-weight: 400;
  }

  .stop-btn:hover:not(:disabled) {
    background: rgba(244, 67, 54, 0.12);
    color: #c62828;
  }

  .launch-more-btn {
    width: 0;
    margin-left: 0;
    opacity: 0;
    overflow: hidden;
    pointer-events: none;
    padding: 0;
  }

  .game-control-group:hover .launch-more-btn,
  .game-control-group:focus-within .launch-more-btn {
    width: 44px;
    margin-left: 8px;
    opacity: 1;
    pointer-events: auto;
  }

  .launch-more-btn:hover:not(:disabled) {
    background: rgba(76, 175, 80, 0.14);
    color: #2e7d32;
    transform: scale(1.04);
  }

  .stop-btn:disabled,
  .launch-more-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: transparent;
    color: var(--text-secondary);
    transition:
      background 0.2s var(--ease-smooth),
      color 0.2s var(--ease-smooth),
      transform 0.2s var(--ease-smooth);
  }

  .add-btn:hover {
    background: var(--control-bg);
    color: var(--text-primary);
    transform: scale(1.05);
  }

  .hint {
    margin-top: 20px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
    max-width: 280px;
  }

  .offline-note {
    margin-top: 16px;
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.85;
    max-width: 300px;
  }
</style>
