import { invoke } from '@tauri-apps/api/core';

export interface ModpackInfo {
  id: string;
  name: string;
  description: string;
  version: string;
  mcVersion: string;
  loader: string;
  iconUrl?: string;
  modCount: number;
  downloadSize: string;
  installed: boolean;
}

export interface LaunchResult {
  success: boolean;
  message?: string;
  sessionId?: string;
  runningCount?: number;
}

export interface InstanceRunState {
  instanceId: string;
  running: boolean;
  sessionCount: number;
  sessions: Array<{
    sessionId: string;
    instanceId: string;
    instanceName: string;
    pid: number;
  }>;
}

export interface InstanceInfo {
  id: string;
  name: string;
  minecraftVersion: string;
  loader: string;
  modpackId?: string;
  iconUrl?: string;
  memoryMb: number;
}

export interface CreateInstanceRequest {
  name: string;
  minecraftVersion: string;
  loader: string;
  modpackId?: string;
  iconUrl?: string;
}

export interface CurseForgeModpackHit {
  id: string;
  slug: string;
  name: string;
  summary: string;
  logoUrl?: string;
  downloadCount: number;
}

export interface AuthResult {
  success: boolean;
  username?: string;
  uuid?: string;
  userCode?: string;
  verificationUri?: string;
  accountId?: string;
  needsReauth?: boolean;
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export function formatInvokeError(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err && typeof err === 'object') {
    const o = err as Record<string, unknown>;
    if (typeof o.message === 'string') return o.message;
    if (typeof o.error === 'string') return o.error;
  }
  if (err instanceof Error) return err.message;
  return 'Unknown error';
}

export async function getSettings(): Promise<Record<string, unknown>> {
  if (!isTauri()) return {};
  return invoke('get_settings');
}

export async function saveSettings(
  settings: Record<string, unknown>
): Promise<void> {
  if (!isTauri()) return;
  return invoke('save_settings', { settings });
}

export async function pickJavaPath(): Promise<string | null> {
  if (!isTauri()) return null;
  return invoke<string | null>('pick_java_path');
}

export async function getMcVersions(): Promise<string[]> {
  if (!isTauri()) return ['1.21.6', '1.21.4', '1.21.1', '1.20.4'];
  return invoke('get_mc_versions');
}

export async function getSystemMemoryMb(): Promise<number> {
  if (!isTauri()) return 16_384;
  return invoke<number>('get_system_memory_mb');
}

export async function listInstances(): Promise<InstanceInfo[]> {
  if (!isTauri()) return [];
  return invoke('list_instances');
}

export async function createInstance(
  request: CreateInstanceRequest
): Promise<InstanceInfo> {
  if (!isTauri()) {
    return {
      id: 'dev-instance',
      name: request.name,
      minecraftVersion: request.minecraftVersion,
      loader: request.loader,
      modpackId: request.modpackId,
      iconUrl: request.iconUrl,
      memoryMb: 4096,
    };
  }
  return invoke('create_instance', { request });
}

export async function launchInstance(instanceId: string): Promise<LaunchResult> {
  if (!isTauri()) return { success: true, message: 'Dev mode launch' };
  return invoke('launch_instance', { instanceId });
}

export async function getInstanceRunState(
  instanceId: string
): Promise<InstanceRunState> {
  if (!isTauri()) {
    return {
      instanceId,
      running: false,
      sessionCount: 0,
      sessions: [],
    };
  }
  return invoke('get_instance_run_state', { instanceId });
}

export async function stopInstanceGame(instanceId: string): Promise<number> {
  if (!isTauri()) return 0;
  return invoke('stop_instance_game', { instanceId });
}

export async function deleteInstance(instanceId: string): Promise<void> {
  if (!isTauri()) return;
  return invoke('delete_instance', { instanceId });
}

export async function searchModpacks(
  mcVersion: string,
  query?: string
): Promise<ModpackInfo[]> {
  if (!isTauri()) return [];
  return invoke('search_modpacks', { mcVersion, query });
}

export async function searchCurseForgeModpacks(
  mcVersion: string,
  query: string
): Promise<CurseForgeModpackHit[]> {
  if (!isTauri()) return [];
  const raw = await invoke<
    Array<{
      id: string;
      slug: string;
      name: string;
      summary: string;
      logo_url?: string;
      download_count?: number;
    }>
  >('search_curseforge_modpacks', { mcVersion, query });
  return raw.map((hit) => ({
    id: hit.id,
    slug: hit.slug,
    name: hit.name,
    summary: hit.summary,
    logoUrl: hit.logo_url,
    downloadCount: hit.download_count ?? 0,
  }));
}

export async function applyDefaultPreset(): Promise<LaunchResult> {
  if (!isTauri()) return { success: true, message: 'Dev mode preset' };
  return invoke('apply_default_preset');
}

export async function loginOffline(username: string): Promise<AuthResult> {
  if (!isTauri()) {
    return { success: true, username, uuid: '00000000-0000-0000-0000-000000000000' };
  }
  return invoke('login_offline', { username });
}

export async function loginMicrosoft(): Promise<AuthResult> {
  if (!isTauri()) {
    return {
      success: true,
      userCode: 'ABCD-1234',
      verificationUri: 'https://microsoft.com/devicelogin',
    };
  }
  return invoke('login_microsoft');
}

export async function loginMicrosoftPoll(): Promise<AuthResult> {
  if (!isTauri()) return { success: false };
  return invoke('login_microsoft_poll');
}

export async function switchAccount(accountId: string): Promise<AuthResult> {
  if (!isTauri()) return { success: true, accountId };
  return invoke('switch_account', { accountId });
}

export async function signOutAccount(): Promise<void> {
  if (!isTauri()) return;
  return invoke('sign_out_account');
}

export async function linkDiscord(): Promise<{ success: boolean }> {
  if (!isTauri()) return { success: true };
  const result = await invoke<Record<string, unknown>>('link_discord');
  return { success: Boolean(result.success) };
}

export async function minimizeWindow(): Promise<void> {
  if (!isTauri()) return;
  return invoke('minimize_window');
}

export async function maximizeWindow(): Promise<void> {
  if (!isTauri()) return;
  return invoke('maximize_window');
}

export async function closeWindow(): Promise<void> {
  if (!isTauri()) return;
  return invoke('close_window');
}

export async function openExternal(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, '_blank');
    return;
  }
  return invoke('open_external', { url });
}
