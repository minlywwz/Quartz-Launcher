
export const DEFAULT_INSTANCE_ICONS = [
  '/defaults/marin-cave.png',
  '/defaults/marin-enderman.png',
  '/defaults/logo-q.png',
] as const;

export function pickRandomDefaultIcon(seed?: string): string {
  const pool = DEFAULT_INSTANCE_ICONS;
  if (pool.length === 0) return '/defaults/logo-q.png';
  if (!seed) {
    return pool[Math.floor(Math.random() * pool.length)]!;
  }
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash * 31 + seed.charCodeAt(i)) >>> 0;
  }
  return pool[hash % pool.length]!;
}

export function resolveInstanceIcon(iconUrl?: string | null, instanceId?: string): string {
  if (iconUrl?.trim()) return iconUrl.trim();
  return pickRandomDefaultIcon(instanceId ?? '');
}

export function isCustomIcon(iconUrl?: string | null): boolean {
  return Boolean(iconUrl?.trim());
}
