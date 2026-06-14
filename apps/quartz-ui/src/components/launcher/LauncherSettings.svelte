<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { settings } from '$lib/stores/settings';
  import { launcherPanel } from '$lib/stores/launcher';
  import { currentView } from '$lib/stores/view';
  import { getSystemMemoryMb, pickJavaPath } from '$lib/api';
  import type { Theme } from '$lib/stores/settings';

  const MEMORY_MIN_MB = 1024;
  const MEMORY_STEP_MB = 512;

  let systemMemoryMb = $state(MEMORY_MIN_MB);
  let maxMemoryMb = $state(MEMORY_MIN_MB);

  function alignToStep(mb: number): number {
    return Math.max(MEMORY_MIN_MB, Math.floor(mb / MEMORY_STEP_MB) * MEMORY_STEP_MB);
  }

  function formatMemoryLabel(mb: number): string {
    if (mb >= 1024) {
      const gb = mb / 1024;
      return Number.isInteger(gb) ? `${gb} GB` : `${gb.toFixed(1)} GB`;
    }
    return `${mb} MB`;
  }

  onMount(async () => {
    try {
      const total = await getSystemMemoryMb();
      systemMemoryMb = total;
      maxMemoryMb = alignToStep(total);
      const current = get(settings).memoryMb;
      if (current > maxMemoryMb) {
        settings.patch({ memoryMb: maxMemoryMb });
      } else if (current < MEMORY_MIN_MB) {
        settings.patch({ memoryMb: MEMORY_MIN_MB });
      }
    } catch (err) {
      console.error('Failed to read system memory:', err);
    }
  });

  function updateSetting<K extends keyof import('$lib/stores/settings').AppSettings>(
    key: K,
    value: import('$lib/stores/settings').AppSettings[K]
  ) {
    settings.patch({ [key]: value } as Partial<import('$lib/stores/settings').AppSettings>);
  }

  function handleThemeChange(theme: Theme) {
    settings.setTheme(theme);
  }

  async function browseJavaPath() {
    const path = await pickJavaPath();
    if (path) updateSetting('javaPath', path);
  }

  const memoryGb = $derived(($settings.memoryMb / 1024).toFixed(1));
</script>

<div class="settings">
  <header class="settings-head">
    <button class="back-btn" type="button" onclick={() => launcherPanel.goHome()} aria-label="Back">
      <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
        <path
          d="M8.5 2.5L4 7l4.5 4.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <h2 class="settings-title">Settings</h2>
  </header>

  <div class="settings-scroll">
    <section class="settings-group">
      <h3 class="group-title">Memory</h3>
      <p class="group-desc">
        RAM allocated to Minecraft (-Xmx). System total: {formatMemoryLabel(systemMemoryMb)}.
      </p>

      <div class="memory-card">
        <div class="memory-value">
          <span class="memory-num">{$settings.memoryMb}</span>
          <span class="memory-unit">MB</span>
          <span class="memory-gb">≈ {memoryGb} GB</span>
        </div>
        <input
          class="memory-slider"
          type="range"
          min={MEMORY_MIN_MB}
          max={maxMemoryMb}
          step={MEMORY_STEP_MB}
          value={$settings.memoryMb}
          oninput={(e) => updateSetting('memoryMb', Number(e.currentTarget.value))}
          aria-label="Allocated memory"
        />
        <div class="memory-labels">
          <span>{formatMemoryLabel(MEMORY_MIN_MB)}</span>
          <span>{formatMemoryLabel(maxMemoryMb)}</span>
        </div>
      </div>
    </section>

    <section class="settings-group">
      <h3 class="group-title">Appearance</h3>
      <div class="theme-row">
        {#each ['light', 'dark', 'system'] as t (t)}
          <button
            class="theme-btn"
            class:active={$settings.theme === t}
            type="button"
            onclick={() => handleThemeChange(t as Theme)}
          >
            {t === 'light' ? 'Light' : t === 'dark' ? 'Dark' : 'System'}
          </button>
        {/each}
      </div>
    </section>

    <section class="settings-group">
      <h3 class="group-title">Launcher</h3>
      <div class="toggle-row">
        <span>Discord Rich Presence</span>
        <label class="vision-toggle">
          <input
            type="checkbox"
            checked={$settings.enableDiscordRpc}
            onchange={(e) => updateSetting('enableDiscordRpc', e.currentTarget.checked)}
          />
          <span class="vision-toggle-track"></span>
        </label>
      </div>
      <div class="toggle-row">
        <span>Minimize launcher when game starts</span>
        <label class="vision-toggle">
          <input
            type="checkbox"
            checked={$settings.closeOnLaunch}
            onchange={(e) => updateSetting('closeOnLaunch', e.currentTarget.checked)}
          />
          <span class="vision-toggle-track"></span>
        </label>
      </div>
      <div class="toggle-row">
        <span>Show snapshot versions</span>
        <label class="vision-toggle">
          <input
            type="checkbox"
            checked={$settings.showSnapshots}
            onchange={(e) => updateSetting('showSnapshots', e.currentTarget.checked)}
          />
          <span class="vision-toggle-track"></span>
        </label>
      </div>
    </section>

    <section class="settings-group">
      <h3 class="group-title">Java</h3>
      <p class="group-desc java-path">
        {$settings.javaPath || 'Automatic — Mojang runtime for the selected Minecraft version'}
      </p>
      <div class="java-actions">
        <button class="mini-btn" type="button" onclick={browseJavaPath}>Browse…</button>
        {#if $settings.javaPath}
          <button class="mini-btn" type="button" onclick={() => updateSetting('javaPath', '')}>
            Automatic
          </button>
        {/if}
      </div>
    </section>

    <section class="settings-group">
      <button class="link-btn" type="button" onclick={() => currentView.goToLoginForSwitch('settings')}>
        Switch account
      </button>
    </section>
  </div>
</div>

<style>
  .settings {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .settings-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px 4px;
    flex-shrink: 0;
  }

  .back-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    color: var(--text-secondary);
    background: var(--control-bg);
  }

  .settings-title {
    font-size: 17px;
    font-weight: 700;
  }

  .settings-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .group-title {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 4px;
  }

  .group-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 12px;
  }

  .memory-card {
    padding: 16px;
    border-radius: var(--control-radius);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
  }

  .memory-value {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin-bottom: 14px;
  }

  .memory-num {
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }

  .memory-unit {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .memory-gb {
    margin-left: auto;
    font-size: 13px;
    color: var(--text-tertiary);
  }

  .memory-slider {
    width: 100%;
    accent-color: var(--accent);
  }

  .memory-labels {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .theme-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .theme-btn {
    padding: 8px 14px;
    border-radius: var(--control-radius-pill);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
  }

  .theme-btn.active {
    color: var(--text-primary);
    background: var(--sidebar-item-active);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 0;
    border-top: 1px solid var(--glass-border-subtle);
    font-size: 14px;
  }

  .toggle-row:first-of-type {
    border-top: none;
  }

  .java-path {
    word-break: break-all;
    margin-bottom: 10px;
  }

  .java-actions {
    display: flex;
    gap: 8px;
  }

  .mini-btn {
    padding: 8px 14px;
    border-radius: var(--control-radius-pill);
    font-size: 13px;
    font-weight: 600;
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
  }

  .link-btn {
    width: 100%;
    text-align: left;
    padding: 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--accent);
  }
</style>
