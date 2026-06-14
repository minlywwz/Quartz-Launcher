<script lang="ts">
  import UserAvatar from '../shell/UserAvatar.svelte';
  import { ChevronDown } from '$lib/icons';
  import type { SavedAccount } from '$lib/stores/settings';

  interface Props {
    displayName: string;
    otherAccounts: SavedAccount[];
    switchingId?: string | null;
    onswitch?: (account: SavedAccount) => void;
    onadd?: () => void;
    onsignout?: () => void;
  }

  let {
    displayName,
    otherAccounts,
    switchingId = null,
    onswitch,
    onadd,
    onsignout,
  }: Props = $props();

  let open = $state(false);

  export function closeMenu() {
    open = false;
  }

  function toggle() {
    open = !open;
  }

  function scrollableLabel(node: HTMLElement) {
    const onWheel = (e: WheelEvent) => {
      if (node.scrollWidth <= node.clientWidth) return;
      e.preventDefault();
      node.scrollLeft += e.deltaY;
    };
    node.addEventListener('wheel', onWheel, { passive: false });
    return {
      destroy() {
        node.removeEventListener('wheel', onWheel);
      },
    };
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (open && e.key === 'Escape') open = false;
  }}
/>

{#if open}
  <button
    class="sheet-backdrop"
    type="button"
    aria-label="Close menu"
    tabindex="-1"
    onclick={() => (open = false)}
  ></button>
{/if}


<div class="profile-sheet" class:open>
  <div class="sheet-anchor">
    <div class="sheet-surface">
      {#if open}
        <div class="sheet-menu" role="menu">
          {#if otherAccounts.length > 0}
            <p class="menu-label">Switch account</p>
            {#each otherAccounts as account (account.id)}
              <button
                type="button"
                role="menuitem"
                class="menu-row"
                onclick={() => onswitch?.(account)}
                disabled={switchingId === account.id}
              >
                <UserAvatar username={account.username} size={22} alt={account.username} />
                <span class="menu-row-text">
                  <span
                    class="scroll-name menu-row-title"
                    use:scrollableLabel
                    title={account.username}
                  >
                    {account.username}
                  </span>
                  <span class="menu-row-sub">
                    {account.microsoftLinked ? 'Microsoft' : 'Offline'}
                  </span>
                </span>
              </button>
            {/each}
          {/if}

          <button type="button" role="menuitem" class="menu-row menu-row-plain" onclick={() => onadd?.()}>
            Add account
          </button>
          <button
            type="button"
            role="menuitem"
            class="menu-row menu-row-plain menu-row-danger"
            onclick={() => onsignout?.()}
          >
            Sign out
          </button>
        </div>
      {/if}

      <button
        class="sheet-trigger"
        class:open
        type="button"
        onclick={toggle}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        <UserAvatar username={displayName} size={28} alt={displayName} />
        <span class="scroll-name sheet-label" use:scrollableLabel title={displayName}>
          {displayName}
        </span>
        <span class="sheet-chevron" class:open>
          <ChevronDown size={12} />
        </span>
      </button>
    </div>
  </div>
</div>

<style>
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    border: none;
    background: transparent;
    cursor: default;
  }

  .profile-sheet {
    position: relative;
    width: var(--profile-sheet-width, 148px);
    height: 44px;
    flex-shrink: 0;
  }

  .profile-sheet.open {
    z-index: 10000;
  }

  .sheet-anchor {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
  }

  .sheet-surface {
    display: flex;
    flex-direction: column;
    border-radius: var(--control-radius-pill);
    border: 1px solid var(--glass-border-subtle);
    background: var(--control-bg);
    overflow: hidden;
    box-shadow: none;
    transition: box-shadow 0.2s cubic-bezier(0.4, 0, 0.15, 1);
  }

  .profile-sheet.open .sheet-surface {
    border-radius: var(--profile-sheet-radius);
    corner-shape: superellipse(1.25);
    background: var(--popover-bg);
    box-shadow: var(--glass-shadow-panel);
  }

  .sheet-menu {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 4px 2px;
    animation: sheet-menu-in 0.22s cubic-bezier(0.4, 0, 0.15, 1);
  }

  @keyframes sheet-menu-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .sheet-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 44px;
    padding: 0 10px;
    box-sizing: border-box;
    border: none;
    border-radius: 0;
    background: transparent;
    font-size: 13px;
    font-weight: 600;
    line-height: 1;
    color: var(--text-primary);
  }

  .sheet-trigger :global(.avatar-img),
  .sheet-trigger :global(.avatar-fallback) {
    display: block;
  }

  .sheet-trigger.open {
    border-top: 1px solid var(--group-divider);
  }

  .sheet-chevron {
    display: flex;
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: transform 0.22s cubic-bezier(0.4, 0, 0.15, 1);
  }

  .sheet-chevron.open {
    transform: rotate(180deg);
  }

  .scroll-name {
    display: block;
    overflow-x: auto;
    overflow-y: hidden;
    white-space: nowrap;
    text-align: left;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
    cursor: default;
  }

  .scroll-name::-webkit-scrollbar {
    display: none;
  }

  .sheet-label {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    line-height: 1;
  }

  .menu-row-text .scroll-name {
    flex: 1;
    min-width: 0;
  }

  .menu-label {
    padding: 6px 10px 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }

  .menu-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--profile-menu-row-radius);
    text-align: left;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    border: none;
    background: transparent;
  }

  .menu-row:hover:not(:disabled) {
    background: var(--row-hover);
  }

  .menu-row:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  .menu-row-plain {
    font-weight: 600;
    padding-left: 12px;
  }

  .menu-row-danger {
    color: var(--danger);
  }

  .menu-row-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
    gap: 1px;
  }

  .menu-row-title {
    font-weight: 600;
  }

  .menu-row-sub {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  @media (prefers-reduced-motion: reduce) {
    .sheet-surface,
    .sheet-chevron {
      transition: none;
    }

    .sheet-menu {
      animation: none;
    }
  }
</style>
