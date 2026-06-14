<script lang="ts">
  import { settings } from '$lib/stores/settings';
  import { currentView } from '$lib/stores/view';
  import { linkDiscord } from '$lib/api';
  import type { Theme, DefaultPreset } from '$lib/stores/settings';
  import PanelHeader from './shell/PanelHeader.svelte';
  import GlassGroup from './shell/GlassGroup.svelte';
  import GlassRow from './shell/GlassRow.svelte';

  const PRESETS: { id: DefaultPreset; name: string; desc: string }[] = [
    {
      id: 'latest-vanilla',
      name: 'Latest Vanilla',
      desc: 'Newest official Minecraft release',
    },
    {
      id: 'latest-fabric-optimized',
      name: 'Latest Fabric + Optimized',
      desc: 'Fabulously Optimized modpack from Modrinth',
    },
  ];

  let step = $state(0);
  let selectedTheme = $state<Theme>('dark');
  let discordEnabled = $state(true);
  let selectedPreset = $state<DefaultPreset>('latest-fabric-optimized');
  let linking = $state(false);

  async function handleDiscordToggle() {
    if (!discordEnabled) return;
    linking = true;
    try {
      await linkDiscord();
    } catch {

    } finally {
      linking = false;
    }
  }

  function next() {
    if (step < 3) {
      step++;
      if (step === 1 && discordEnabled) handleDiscordToggle();
    } else {
      finish();
    }
  }

  function back() {
    if (step > 0) step--;
  }

  function finish() {
    settings.completeOnboarding({
      theme: selectedTheme,
      enableDiscordRpc: discordEnabled,
      defaultPreset: selectedPreset,
    });
    currentView.goToLogin();
  }

  function skipToLogin() {
    settings.completeOnboarding({ theme: selectedTheme });
    currentView.goToLogin();
  }
</script>

<div class="wizard">
  <PanelHeader title="Welcome" />

  <div class="wizard-scroll">
    {#if step === 0}
      <GlassGroup label="Appearance" footer="You can change this later in Settings.">
        <GlassRow
          label="Light"
          trailing={selectedTheme === 'light' ? 'check' : 'none'}
          active={selectedTheme === 'light'}
          onclick={() => (selectedTheme = 'light')}
        />
        <GlassRow
          label="Dark"
          trailing={selectedTheme === 'dark' ? 'check' : 'none'}
          active={selectedTheme === 'dark'}
          onclick={() => (selectedTheme = 'dark')}
        />
        <GlassRow
          label="System"
          trailing={selectedTheme === 'system' ? 'check' : 'none'}
          active={selectedTheme === 'system'}
          onclick={() => (selectedTheme = 'system')}
        />
      </GlassGroup>
    {:else if step === 1}
      <GlassGroup label="Integrations">
        <GlassRow label="Discord Rich Presence" subtitle="Show what you're playing" trailing="none">
          {#snippet accessory()}
            <label class="vision-toggle">
              <input type="checkbox" bind:checked={discordEnabled} disabled={linking} />
              <span class="vision-toggle-track"></span>
            </label>
          {/snippet}
        </GlassRow>
      </GlassGroup>
    {:else if step === 2}
      <GlassGroup label="Default instance">
        {#each PRESETS as preset}
          <GlassRow
            label={preset.name}
            subtitle={preset.desc}
            trailing={selectedPreset === preset.id ? 'check' : 'chevron'}
            active={selectedPreset === preset.id}
            onclick={() => (selectedPreset = preset.id)}
          />
        {/each}
      </GlassGroup>
    {:else}
      <GlassGroup label="Account" footer="You'll sign in on the next screen.">
        <GlassRow label="Offline username or Microsoft account" trailing="chevron" />
      </GlassGroup>
    {/if}

    <div class="wizard-actions">
      <button class="btn btn-ghost" type="button" onclick={skipToLogin}>Skip</button>
      <div class="wizard-actions-right">
        {#if step > 0}
          <button class="btn btn-secondary" type="button" onclick={back}>Back</button>
        {/if}
        <button class="btn btn-primary" type="button" onclick={next}>
          {step === 3 ? 'Finish' : 'Continue'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .wizard {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .wizard-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 22px 28px;
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .wizard-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 8px;
  }

  .wizard-actions-right {
    display: flex;
    gap: 8px;
  }
</style>
