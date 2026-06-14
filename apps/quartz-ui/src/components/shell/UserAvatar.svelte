<script lang="ts">
  import { minecraftAvatarUrl } from '$lib/minecraftAvatar';

  interface Props {
    username?: string;
    size?: number;
    alt?: string;
  }

  let { username = '', size = 36, alt = 'Player avatar' }: Props = $props();

  const src = $derived(
    username.trim().length > 0 ? minecraftAvatarUrl(username.trim(), size) : null
  );

  const initial = $derived(
    username.trim().length > 0 ? username.trim().slice(0, 1).toUpperCase() : '?'
  );

  let failed = $state(false);

  $effect(() => {
    username;
    failed = false;
  });
</script>

{#if src && !failed}
  <img
    class="avatar-img"
    {src}
    {alt}
    width={size}
    height={size}
    loading="lazy"
    decoding="async"
    onerror={() => (failed = true)}
  />
{:else}
  <span class="avatar-fallback" style:width="{size}px" style:height="{size}px" aria-hidden="true">
    {initial}
  </span>
{/if}

<style>
  .avatar-img,
  .avatar-fallback {
    border-radius: 50%;
    flex-shrink: 0;
    object-fit: cover;
  }

  .avatar-fallback {
    display: grid;
    place-items: center;
    font-size: calc(var(--size, 36px) * 0.38);
    font-weight: 700;
    color: var(--text-inverse);
    background: var(--accent);
  }

  .avatar-fallback {
    font-size: 13px;
  }
</style>
