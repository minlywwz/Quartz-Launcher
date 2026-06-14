<script lang="ts">
  import type { InstanceInfo, ModpackInfo, CurseForgeModpackHit } from '$lib/api';
  import {
    createInstance,
    getMcVersions,
    searchModpacks,
    searchCurseForgeModpacks,
    formatInvokeError,
  } from '$lib/api';
  import { settings } from '$lib/stores/settings';
  import { launcherPanel } from '$lib/stores/launcher';
  import { pickRandomDefaultIcon } from '$lib/assets';
  import SegmentedControl from './SegmentedControl.svelte';
  import InstanceHeroIcon from './InstanceHeroIcon.svelte';

  interface Props {
    oncreated?: (instance: InstanceInfo) => void;
  }

  let { oncreated }: Props = $props();

  type BrowseTab = 'create' | 'modrinth' | 'curseforge';

  const tabs = [
    { id: 'create', label: 'Create' },
    { id: 'modrinth', label: 'Modrinth' },
    { id: 'curseforge', label: 'CurseForge' },
  ] as const;

  let tab = $state<BrowseTab>('create');
  let versions = $state<string[]>([]);
  let selectedVersion = $state('1.21.4');
  let selectedLoader = $state('fabric');
  let instanceName = $state('');
  let customIconUrl = $state('');
  let modrinthQuery = $state('');
  let modrinthResults = $state<ModpackInfo[]>([]);
  let selectedModrinth = $state<ModpackInfo | null>(null);
  let cfQuery = $state('');
  let cfResults = $state<CurseForgeModpackHit[]>([]);
  let selectedCf = $state<CurseForgeModpackHit | null>(null);
  let loadingModrinth = $state(false);
  let loadingCf = $state(false);
  let creating = $state(false);
  let error = $state('');
  let modrinthError = $state('');
  let cfError = $state('');

  const previewIcon = $derived(
    customIconUrl.trim() ||
      selectedModrinth?.iconUrl ||
      selectedCf?.logoUrl ||
      pickRandomDefaultIcon(instanceName || 'new-build')
  );

  const LOADERS = [
    { id: 'vanilla', label: 'Vanilla' },
    { id: 'fabric', label: 'Fabric' },
    { id: 'forge', label: 'Forge' },
    { id: 'neoforge', label: 'NeoForge' },
  ];

  $effect(() => {
    void loadVersions();
  });

  async function loadVersions() {
    try {
      versions = await getMcVersions();
      selectedVersion = versions.find((v) => v.startsWith('1.')) ?? versions[0] ?? '1.21.4';
    } catch {
      versions = ['1.21.4', '1.21.1', '1.20.4'];
      selectedVersion = '1.20.4';
    }
  }

  async function searchModrinth() {
    loadingModrinth = true;
    modrinthError = '';
    try {
      modrinthResults = await searchModpacks(selectedVersion, modrinthQuery.trim() || undefined);
    } catch (e) {
      modrinthResults = [];
      modrinthError = formatInvokeError(e);
    } finally {
      loadingModrinth = false;
    }
  }

  async function searchCf() {
    loadingCf = true;
    cfError = '';
    try {
      cfResults = await searchCurseForgeModpacks(selectedVersion, cfQuery.trim());
      if (cfResults.length === 0 && cfQuery.trim()) {
        cfError = 'No modpacks found. Try another search or Minecraft version.';
      }
    } catch (e) {
      cfResults = [];
      cfError = formatInvokeError(e);
      if (cfError.includes('CURSEFORGE')) {
        cfError = 'Add CURSEFORGE_API_KEY to .env and restart the launcher.';
      }
    } finally {
      loadingCf = false;
    }
  }

  $effect(() => {
    if (tab === 'modrinth' && selectedVersion) {
      const t = setTimeout(() => void searchModrinth(), 300);
      return () => clearTimeout(t);
    }
  });

  $effect(() => {
    if (tab === 'curseforge' && selectedVersion) {
      const t = setTimeout(() => void searchCf(), 300);
      return () => clearTimeout(t);
    }
  });

  function pickModrinth(pack: ModpackInfo) {
    selectedModrinth = pack;
    instanceName = pack.name;
    customIconUrl = pack.iconUrl ?? '';
  }

  function pickCf(hit: CurseForgeModpackHit) {
    selectedCf = hit;
    instanceName = hit.name;
    customIconUrl = hit.logoUrl ?? '';
  }

  async function handleCreate() {
    error = '';
    const name = instanceName.trim();
    if (!name) {
      error = 'Enter an instance name';
      return;
    }

    if (tab === 'modrinth' && !selectedModrinth) {
      error = 'Select a modpack from Modrinth';
      return;
    }

    if (tab === 'curseforge') {
      error = 'CurseForge modpack installation is not available yet. Use Modrinth to install modpacks.';
      return;
    }

    creating = true;
    try {
      let loader = selectedLoader;
      let modpackId: string | undefined;

      if (tab === 'modrinth' && selectedModrinth) {
        modpackId = selectedModrinth.id;
        loader = selectedModrinth.loader !== 'unknown' ? selectedModrinth.loader : 'fabric';
      } else if (selectedLoader === 'vanilla') {
        modpackId = `vanilla:${selectedVersion}`;
        loader = 'vanilla';
      }

      const icon =
        customIconUrl.trim() ||
        selectedModrinth?.iconUrl ||
        selectedCf?.logoUrl ||
        pickRandomDefaultIcon(`${name}-${selectedVersion}-${Date.now()}`);

      const instance = await createInstance({
        name,
        minecraftVersion: selectedVersion,
        loader,
        modpackId,
        iconUrl: icon,
      });

      settings.patch({ lastSelectedInstanceId: instance.id });
      oncreated?.(instance);
      launcherPanel.goHome();
    } catch (e) {
      error = formatInvokeError(e);
    } finally {
      creating = false;
    }
  }

  function goBack() {
    launcherPanel.goHome();
  }
</script>

<div class="browse">
  <header class="browse-head">
    <button class="back-btn" type="button" onclick={goBack} aria-label="Back">
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
    <h2 class="browse-title">Instances</h2>
  </header>

  <div class="browse-body">
    <SegmentedControl
      options={[...tabs]}
      value={tab}
      onchange={(id) => (tab = id as BrowseTab)}
      ariaLabel="Add instance mode"
    />

    {#if tab === 'create'}
      <div class="form-section">
        <div class="preview-row">
          <InstanceHeroIcon
            iconUrl={previewIcon}
            name={instanceName || 'Instance'}
            size={72}
          />
          <div class="preview-fields">
            <label class="field">
              <span class="field-label">Name</span>
              <input class="field-input" type="text" bind:value={instanceName} placeholder="My instance" />
            </label>
          </div>
        </div>

        <label class="field">
          <span class="field-label">Minecraft version</span>
          <select class="field-select" bind:value={selectedVersion}>
            {#each versions as v (v)}
              <option value={v}>{v}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="field-label">Loader</span>
          <select class="field-select" bind:value={selectedLoader}>
            {#each LOADERS as l (l.id)}
              <option value={l.id}>{l.label}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="field-label">Icon URL (optional)</span>
          <input
            class="field-input"
            type="url"
            bind:value={customIconUrl}
            placeholder="https://…"
          />
        </label>
      </div>
    {:else if tab === 'modrinth'}
      <div class="form-section">
        <label class="field">
          <span class="field-label">Minecraft version</span>
          <select class="field-select" bind:value={selectedVersion}>
            {#each versions as v (v)}
              <option value={v}>{v}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="field-label">Search Modrinth</span>
          <input
            class="field-input"
            type="search"
            bind:value={modrinthQuery}
            placeholder="Modpack name…"
            onkeydown={(e) => e.key === 'Enter' && searchModrinth()}
          />
        </label>

        <div class="pack-list">
          {#if loadingModrinth}
            <p class="muted">Loading…</p>
          {:else if modrinthError}
            <p class="error">{modrinthError}</p>
          {:else if modrinthResults.length === 0}
            <p class="muted">No modpacks found for this version</p>
          {:else}
            {#each modrinthResults as pack (pack.id)}
              <button
                class="pack-item"
                class:active={selectedModrinth?.id === pack.id}
                type="button"
                onclick={() => pickModrinth(pack)}
              >
                {#if pack.iconUrl}
                  <img class="pack-icon" src={pack.iconUrl} alt="" width="40" height="40" />
                {:else}
                  <span class="pack-icon-fallback">{pack.name.slice(0, 1)}</span>
                {/if}
                <span class="pack-meta">
                  <span class="pack-name">{pack.name}</span>
                  <span class="pack-sub">{pack.loader} · {pack.description.slice(0, 80)}</span>
                </span>
              </button>
            {/each}
          {/if}
        </div>

        {#if selectedModrinth}
          <p class="selected-label">Selected: {selectedModrinth.name}</p>
        {/if}
      </div>
    {:else}
      <div class="form-section">
        <label class="field">
          <span class="field-label">Minecraft version</span>
          <select class="field-select" bind:value={selectedVersion}>
            {#each versions as v (v)}
              <option value={v}>{v}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span class="field-label">Search CurseForge</span>
          <input
            class="field-input"
            type="search"
            bind:value={cfQuery}
            placeholder="Modpack name…"
            onkeydown={(e) => e.key === 'Enter' && searchCf()}
          />
        </label>

        <div class="pack-list">
          {#if loadingCf}
            <p class="muted">Loading…</p>
          {:else if cfError}
            <p class="error">{cfError}</p>
          {:else if cfResults.length === 0}
            <p class="muted">Enter a search term to browse CurseForge modpacks</p>
          {:else}
            {#each cfResults as hit (hit.id)}
              <button
                class="pack-item"
                class:active={selectedCf?.id === hit.id}
                type="button"
                onclick={() => pickCf(hit)}
              >
                {#if hit.logoUrl}
                  <img class="pack-icon" src={hit.logoUrl} alt="" width="40" height="40" />
                {:else}
                  <span class="pack-icon-fallback">{hit.name.slice(0, 1)}</span>
                {/if}
                <span class="pack-meta">
                  <span class="pack-name">{hit.name}</span>
                  <span class="pack-sub">{hit.summary.slice(0, 80)}</span>
                </span>
              </button>
            {/each}
          {/if}
        </div>

        {#if selectedCf}
          <p class="selected-label">Selected: {selectedCf.name}</p>
        {/if}
        <p class="cf-note">
          Browse only — CurseForge installation requires CURSEFORGE_API_KEY and is not implemented yet.
          Use Modrinth to install modpacks.
        </p>
      </div>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </div>

  <footer class="browse-foot">
    <button
      class="install-btn"
      type="button"
      disabled={creating || tab === 'curseforge'}
      onclick={handleCreate}
    >
      {#if creating}
        Installing…
      {:else if tab === 'modrinth'}
        Install modpack
      {:else if tab === 'curseforge'}
        Install unavailable
      {:else}
        Create instance
      {/if}
    </button>
  </footer>
</div>

<style>
  .browse {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .browse-head {
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

  .browse-title {
    font-size: 17px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .browse-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 20px 8px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .preview-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px;
    border-radius: var(--control-radius);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
  }

  .preview-fields {
    flex: 1;
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .field-input,
  .field-select {
    padding: 10px 12px;
    border-radius: var(--control-radius-sm);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
    color: var(--text-primary);
    font-size: 14px;
  }

  .pack-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 220px;
    overflow-y: auto;
  }

  .pack-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--control-radius-sm);
    text-align: left;
    transition: background 0.15s var(--ease-smooth);
  }

  .pack-item:hover {
    background: var(--row-hover);
  }

  .pack-item.active {
    background: var(--sidebar-item-active);
    box-shadow: inset 0 0 0 1px var(--glass-border-subtle);
  }

  .pack-icon,
  .pack-icon-fallback {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    flex-shrink: 0;
    object-fit: cover;
  }

  .pack-icon-fallback {
    display: grid;
    place-items: center;
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: 700;
  }

  .pack-meta {
    flex: 1;
    min-width: 0;
  }

  .pack-name {
    display: block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .pack-sub {
    display: block;
    font-size: 11px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selected-label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .cf-note {
    font-size: 11px;
    color: var(--text-tertiary);
    line-height: 1.4;
  }

  .muted {
    font-size: 13px;
    color: var(--text-tertiary);
    padding: 8px 0;
  }

  .error {
    font-size: 12px;
    color: var(--danger);
    line-height: 1.4;
    padding: 8px 10px;
    border-radius: var(--control-radius-sm);
    background: rgba(255, 59, 48, 0.1);
  }

  .browse-foot {
    padding: 12px 20px 16px;
    flex-shrink: 0;
    border-top: 1px solid var(--glass-border-subtle);
  }

  .install-btn {
    width: 100%;
    padding: 14px;
    border-radius: var(--control-radius-pill);
    background: var(--accent);
    color: var(--text-inverse);
    font-size: 15px;
    font-weight: 600;
    transition: opacity 0.2s;
  }

  .install-btn:disabled {
    opacity: 0.55;
  }
</style>
