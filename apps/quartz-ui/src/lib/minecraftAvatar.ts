
export function minecraftAvatarUrl(username: string, size = 64): string {
  const safe = encodeURIComponent(username.trim());
  return `https://mc-heads.net/avatar/${safe}/${size}`;
}
