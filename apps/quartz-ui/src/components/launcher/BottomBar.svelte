<script lang="ts">
  import type { InstanceInfo } from '$lib/api';
  import { switchAccount, signOutAccount } from '$lib/api';
  import {
    settings,
    isLoggedIn,
    activeAccount,
    otherAccounts,
    type SavedAccount,
  } from '$lib/stores/settings';
  import { launcherPanel } from '$lib/stores/launcher';
  import { currentView } from '$lib/stores/view';
  import { Gear, ChevronDown } from '$lib/icons';
  import SegmentedControl from './SegmentedControl.svelte';
  import ProfileAccountControl from './ProfileAccountControl.svelte';

  interface Props {
    instances: InstanceInfo[];
    selectedInstance: InstanceInfo | null;
    onselect?: (instance: InstanceInfo) => void;
  }

  let { instances, selectedInstance, onselect }: Props = $props();

  let profileControl = $state<{ closeMenu: () => void } | undefined>();
  let instanceOpen = $state(false);
  let switchingId = $state<string | null>(null);

  const displayName = $derived(
    $activeAccount?.username ||
      $settings.offlineUsername ||
      ($settings.microsoftLinked ? 'Microsoft' : 'Player')
  );

  const savedOthers = $derived(otherAccounts($settings));
  const useSegmented = $derived(instances.length > 0 && instances.length <= 3);
  const segmentOptions = $derived(
    instances.map((i) => ({
      id: i.id,
      label: i.minecraftVersion,
    }))
  );

  function selectInstance(instance: InstanceInfo) {
    onselect?.(instance);
    settings.patch({ lastSelectedInstanceId: instance.id });
    instanceOpen = false;
  }

  function openSettings() {
    launcherPanel.goSettings();
  }

  function closeMenus() {
    profileControl?.closeMenu();
    instanceOpen = false;
  }

  function openSignIn() {
    closeMenus();
    currentView.goToLogin();
  }

  function openAddAccount() {
    closeMenus();
    currentView.goToLoginForSwitch('home');
  }

  async function handleSelectAccount(account: SavedAccount) {
    if (account.id === $settings.activeAccountId) {
      closeMenus();
      return;
    }

    switchingId = account.id;
    try {
      const result = await switchAccount(account.id);
      if (result.needsReauth) {
        closeMenus();
        currentView.goToLoginForSwitch('home');
        return;
      }
      if (result.success) {
        await settings.reloadFromBackend();
        closeMenus();
      }
    } catch (err) {
      console.error('Failed to switch account:', err);
    } finally {
      switchingId = null;
    }
  }

  async function handleSignOut() {
    try {
      await signOutAccount();
      await settings.reloadFromBackend();
    } catch (err) {
      console.error('Failed to sign out:', err);
    } finally {
      closeMenus();
    }
  }

  function handleSegmentChange(id: string) {
    const inst = instances.find((i) => i.id === id);
    if (inst) selectInstance(inst);
  }
</script>

<footer class="bottom-bar">
  <div class="bar-item bar-profile">
    {#if $isLoggedIn}
      <ProfileAccountControl
        bind:this={profileControl}
        {displayName}
        otherAccounts={savedOthers}
        {switchingId}
        onswitch={handleSelectAccount}
        onadd={openAddAccount}
        onsignout={handleSignOut}
      />
    {:else}
      <button class="pill-btn sign-in-btn" type="button" onclick={openSignIn}>Sign in</button>
    {/if}
  </div>

  <div class="bar-item bar-instance">
    {#if useSegmented}
      <SegmentedControl
        options={segmentOptions}
        value={selectedInstance?.id ?? segmentOptions[0]?.id ?? ''}
        onchange={handleSegmentChange}
        ariaLabel="Instance picker"
      />
    {:else}
      <button
        class="pill-btn pill-center"
        type="button"
        disabled={instances.length === 0}
        onclick={() => {
          instanceOpen = !instanceOpen;
          profileControl?.closeMenu();
        }}
        aria-expanded={instanceOpen}
      >
        <span class="pill-label">{selectedInstance?.minecraftVersion ?? 'No instances'}</span>
        <ChevronDown size={12} />
      </button>
      {#if instanceOpen && instances.length > 0}
        <div class="popover popover-center" role="menu">
          {#each instances as inst (inst.id)}
            <button
              type="button"
              role="menuitem"
              class:active={selectedInstance?.id === inst.id}
              onclick={() => selectInstance(inst)}
            >
              <span class="inst-name">{inst.name}</span>
              <span class="inst-ver">{inst.minecraftVersion}</span>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

  <div class="bar-item bar-settings">
    <button class="icon-btn" type="button" aria-label="Settings" onclick={openSettings}>
      <Gear size={20} />
    </button>
  </div>
</footer>

{#if instanceOpen}
  <button class="backdrop" type="button" aria-label="Close menu" onclick={closeMenus}></button>
{/if}

<style>
  .bottom-bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 16px;
    border-top: 1px solid var(--glass-border-subtle);
    background: var(--bottom-bar-bg);
    flex-shrink: 0;
    position: relative;
    z-index: 1000;
    overflow: visible;
  }

  .bar-item {
    position: relative;
    min-width: 0;
    overflow: visible;
  }

  .bar-profile {
    justify-self: start;
    max-width: min(148px, 42vw);
    z-index: 2;
  }

  .bar-instance {
    justify-self: center;
    width: min(100%, 280px);
  }

  .bar-settings {
    justify-self: end;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 9998;
    background: transparent;
    border: none;
    cursor: default;
  }

  .pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--control-radius-pill);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    max-width: 100%;
    box-sizing: border-box;
  }

  .pill-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sign-in-btn {
    color: var(--accent);
  }

  .pill-center {
    min-width: 120px;
    justify-content: center;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: var(--control-radius-sm);
    color: var(--text-secondary);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
    margin-left: auto;
  }

  .icon-btn:hover {
    color: var(--text-primary);
    background: var(--control-bg-hover);
  }

  .popover {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    min-width: 220px;
    padding: 6px;
    border-radius: var(--control-radius);
    background: var(--popover-bg);
    border: 1px solid var(--glass-border-subtle);
    box-shadow: var(--glass-shadow-panel);
    z-index: 1001;
    animation: popover-in 0.16s ease-out;
    transform-origin: bottom left;
  }

  @keyframes popover-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .popover-center {
    left: 50%;
    transform: translateX(-50%);
    min-width: 220px;
    transform-origin: bottom center;
    animation-name: popover-in-center;
  }

  @keyframes popover-in-center {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  .popover button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 10px 12px;
    border-radius: var(--control-radius-sm);
    text-align: left;
    font-size: 13px;
    color: var(--text-primary);
  }

  .popover button:hover,
  .popover button.active {
    background: var(--row-hover);
  }

  .inst-name {
    font-weight: 600;
  }

  .inst-ver {
    font-size: 11px;
    color: var(--text-tertiary);
  }
</style>
