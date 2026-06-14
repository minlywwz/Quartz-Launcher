<script lang="ts">
  import { resolveInstanceIcon } from '$lib/assets';

  interface Props {
    iconUrl?: string | null;
    instanceId?: string;
    name?: string;
    size?: number;
  }

  let { iconUrl = null, instanceId, name = 'Minecraft', size = 120 }: Props = $props();

  const src = $derived(resolveInstanceIcon(iconUrl, instanceId));
</script>

<img
  class="hero-img"
  src={src}
  alt={name}
  width={size}
  height={size}
  style="--hero-size: {size}px"
/>

<style>
  .hero-img {
    width: var(--hero-size);
    height: var(--hero-size);
    max-width: 140px;
    max-height: 140px;
    object-fit: cover;
    border-radius: 18px;
    filter: drop-shadow(0 12px 18px rgba(0, 0, 0, 0.15));
  }
</style>
