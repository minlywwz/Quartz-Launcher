<script lang="ts">
  import { get } from 'svelte/store';
  import { settings, isLoggedIn } from '$lib/stores/settings';
  import { currentView, loginCancellable, loginReturnPanel } from '$lib/stores/view';
  import { launcherPanel } from '$lib/stores/launcher';
  import PanelHeader from './shell/PanelHeader.svelte';
  import GlassGroup from './shell/GlassGroup.svelte';
  import GlassRow from './shell/GlassRow.svelte';
  import {
    loginOffline,
    loginMicrosoft,
    loginMicrosoftPoll,
    openExternal,
    applyDefaultPreset,
  } from '$lib/api';

  let username = $state('');
  let loading = $state(false);
  let error = $state('');
  let msaCode = $state<string | null>(null);
  let msaUri = $state<string | null>(null);
  let polling = $state(false);
  let pollGeneration = 0;

  const canCancel = $derived($loginCancellable && $isLoggedIn);

  function handleCancel() {
    pollGeneration += 1;
    polling = false;
    loading = false;
    error = '';
    msaCode = null;
    msaUri = null;

    const returnPanel = get(loginReturnPanel);
    currentView.cancelLogin();
    if (returnPanel === 'settings') {
      launcherPanel.goSettings();
    } else {
      launcherPanel.goHome();
    }
  }

  async function afterLogin() {
    const s = get(settings);
    if (s.onboardingComplete && !s.presetApplied) {
      try {
        await applyDefaultPreset();
        settings.patch({ presetApplied: true });
      } catch {

      }
    }
    currentView.goToMain();
    launcherPanel.goHome();
  }

  async function handleOfflineLogin() {
    if (!username.trim()) {
      error = 'Enter a username';
      return;
    }
    loading = true;
    error = '';
    try {
      const result = await loginOffline(username.trim());
      if (result.success) {
        await settings.reloadFromBackend();
        await afterLogin();
      } else {
        error = 'Login failed';
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleMicrosoftLogin() {
    loading = true;
    error = '';
    msaCode = null;
    msaUri = null;
    try {
      const result = await loginMicrosoft();
      if (result.success && result.userCode && result.verificationUri) {
        msaCode = result.userCode;
        msaUri = result.verificationUri;
        polling = true;
        pollMsa();
      } else {
        error = 'Microsoft sign-in unavailable. Set AZURE_CLIENT_ID in .env and restart.';
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pollMsa() {
    const generation = ++pollGeneration;
    const maxAttempts = 120;
    for (let i = 0; i < maxAttempts; i++) {
      if (generation !== pollGeneration) return;
      await new Promise((r) => setTimeout(r, 3000));
      if (generation !== pollGeneration) return;
      try {
        const result = await loginMicrosoftPoll();
        if (generation !== pollGeneration) return;
        if (result.success && result.username) {
          await settings.reloadFromBackend();
          polling = false;
          await afterLogin();
          return;
        }
      } catch {

      }
    }
    if (generation !== pollGeneration) return;
    polling = false;
    error = 'Microsoft sign-in timed out';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleOfflineLogin();
  }
</script>

<div class="login-view">
  <PanelHeader title="Account" showBack={canCancel} onback={handleCancel} />

  <div class="login-scroll">
    <GlassGroup label="Microsoft account">
      <GlassRow
        label="Sign in with Microsoft"
        subtitle={polling ? 'Waiting for approval…' : 'Multiplayer, Realms, and online-mode servers'}
        trailing="chevron"
        onclick={handleMicrosoftLogin}
        disabled={loading || polling}
      />
    </GlassGroup>

    {#if msaCode && msaUri}
      <GlassGroup label="Device code">
        <div class="msa-block">
          <p>
            Open
            <button class="link" type="button" onclick={() => openExternal(msaUri!)}>
              microsoft.com/devicelogin
            </button>
            and enter this code:
          </p>
          <code>{msaCode}</code>
        </div>
      </GlassGroup>
    {/if}

    <GlassGroup
      label="Offline play"
      footer="Offline UUID is deterministic — username case matters for server whitelists."
    >
      <div class="group-field">
        <span class="field-label">Username</span>
        <input
          type="text"
          placeholder="Steve"
          bind:value={username}
          onkeydown={handleKeydown}
          disabled={loading || polling}
          maxlength={16}
        />
      </div>
      <GlassRow
        label={loading ? 'Signing in…' : 'Continue offline'}
        trailing="chevron"
        onclick={handleOfflineLogin}
        disabled={loading || polling}
      />
    </GlassGroup>

    {#if error}
      <p class="group-error" role="alert">{error}</p>
    {/if}

    {#if canCancel}
      <button class="cancel-btn" type="button" onclick={handleCancel} disabled={loading}>
        Cancel
      </button>
    {/if}
  </div>
</div>

<style>
  .login-view {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .login-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 22px 28px;
    display: flex;
    flex-direction: column;
    gap: 22px;
    max-width: 560px;
  }

  .msa-block {
    padding: 14px;
    text-align: center;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .msa-block code {
    display: block;
    margin-top: 8px;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--accent);
  }

  .link {
    color: var(--accent);
    text-decoration: underline;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .group-error {
    font-size: 13px;
    color: var(--danger);
    padding: 0 4px;
  }

  .cancel-btn {
    align-self: center;
    margin-top: 4px;
    padding: 10px 20px;
    border-radius: var(--control-radius-pill);
    font-size: 14px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--control-bg);
    border: 1px solid var(--glass-border-subtle);
  }

  .cancel-btn:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--control-bg-hover);
  }

  .cancel-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
