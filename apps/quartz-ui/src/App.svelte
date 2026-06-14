<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import LauncherShell from './components/launcher/LauncherShell.svelte';
  import OnboardingWizard from './components/OnboardingWizard.svelte';
  import LoginView from './components/LoginView.svelte';
  import { currentView } from '$lib/stores/view';
  import { settings } from '$lib/stores/settings';
  import { listInstances } from '$lib/api';
  import type { InstanceInfo } from '$lib/api';

  let ready = $state(false);
  let instances = $state<InstanceInfo[]>([]);
  let selectedInstance = $state<InstanceInfo | null>(null);
  let instancesLoading = $state(true);

  onMount(async () => {
    await settings.init();
    const s = get(settings);

    if (!s.onboardingComplete) {
      currentView.goToOnboarding();
    } else {
      const loggedIn =
        (s.activeAccountId && s.accounts.some((a) => a.id === s.activeAccountId)) ||
        s.offlineUsername.length > 0 ||
        s.microsoftLinked;

      if (!loggedIn) {
        currentView.goToLogin();
      } else {
        currentView.goToMain();
      }
    }

    ready = true;
  });

  async function loadInstances() {
    instancesLoading = true;
    try {
      instances = await listInstances();
      const savedId = get(settings).lastSelectedInstanceId;
      const picked =
        instances.find((i) => i.id === savedId) ??
        instances.find((i) => i.id === selectedInstance?.id) ??
        instances[0] ??
        null;
      selectedInstance = picked;
      if (picked) {
        settings.patch({ lastSelectedInstanceId: picked.id });
      }
    } catch (e) {
      console.error('failed to load instances', e);
    } finally {
      instancesLoading = false;
    }
  }

  function handleSelectInstance(instance: InstanceInfo) {
    selectedInstance = instance;
    settings.patch({ lastSelectedInstanceId: instance.id });
  }

  function handleInstanceCreated(instance: InstanceInfo) {
    selectedInstance = instance;
    settings.patch({ lastSelectedInstanceId: instance.id });
    void loadInstances();
  }

  $effect(() => {
    if (ready && $currentView === 'main') {
      void loadInstances();
    }
  });
</script>

{#if ready}
  {#if $currentView === 'onboarding'}
    <div class="flow-screen">
      <OnboardingWizard />
    </div>
  {:else if $currentView === 'login'}
    <div class="flow-screen">
      <LoginView />
    </div>
  {:else}
    <LauncherShell
      {instances}
      {selectedInstance}
      instancesReady={!instancesLoading}
      onselectinstance={handleSelectInstance}
      oncreated={handleInstanceCreated}
    />
  {/if}
{:else}
  <div class="boot">Quartz Launcher</div>
{/if}

<style>
  .boot,
  .flow-screen {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--launcher-bg);
  }

  .boot {
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-weight: 600;
  }
</style>
