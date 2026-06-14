import { writable, derived, get } from 'svelte/store';

import { getSettings, saveSettings, loginOffline } from '$lib/api';

export type Theme = 'light' | 'dark' | 'system';

export type DefaultPreset = 'latest-vanilla' | 'latest-fabric-optimized';

export interface SavedAccount {

  id: string;

  username: string;

  uuid?: string;

  microsoftLinked: boolean;

}

export interface AppSettings {

  theme: Theme;

  enableDiscordRpc: boolean;

  defaultPreset: DefaultPreset;

  offlineUsername: string;

  microsoftLinked: boolean;

  accounts: SavedAccount[];

  activeAccountId: string;

  onboardingComplete: boolean;

  presetApplied: boolean;

  javaPath: string;

  memoryMb: number;

  closeOnLaunch: boolean;

  showSnapshots: boolean;

  lastSelectedInstanceId: string;

}

const DEFAULT_SETTINGS: AppSettings = {

  theme: 'light',

  enableDiscordRpc: true,

  defaultPreset: 'latest-fabric-optimized',

  offlineUsername: '',

  microsoftLinked: false,

  accounts: [],

  activeAccountId: '',

  onboardingComplete: false,

  presetApplied: false,

  javaPath: '',

  memoryMb: 4096,

  closeOnLaunch: false,

  showSnapshots: false,

  lastSelectedInstanceId: '',

};

function applyTheme(theme: Theme) {

  const resolved =

    theme === 'system'

      ? window.matchMedia('(prefers-color-scheme: dark)').matches

        ? 'dark'

        : 'light'

      : theme;

  document.documentElement.setAttribute('data-theme', resolved);

}

function parseAccounts(raw: unknown): SavedAccount[] {

  if (!Array.isArray(raw)) return [];

  return raw

    .map((entry) => {

      if (!entry || typeof entry !== 'object') return null;

      const o = entry as Record<string, unknown>;

      const id = typeof o.id === 'string' ? o.id : '';

      const username = typeof o.username === 'string' ? o.username : '';

      if (!id || !username) return null;

      return {

        id,

        username,

        uuid: typeof o.uuid === 'string' ? o.uuid : undefined,

        microsoftLinked: Boolean(o.microsoftLinked),

      } satisfies SavedAccount;

    })

    .filter((a): a is SavedAccount => a !== null);

}

function migrateLegacyAccounts(raw: Record<string, unknown>): SavedAccount[] {

  const existing = parseAccounts(raw.accounts);

  if (existing.length > 0) return existing;

  const username = typeof raw.offlineUsername === 'string' ? raw.offlineUsername.trim() : '';

  if (!username) return [];

  return [

    {

      id: crypto.randomUUID(),

      username,

      microsoftLinked: Boolean(raw.microsoftLinked),

    },

  ];

}

function fromBackend(raw: Record<string, unknown>): AppSettings {

  const accounts = migrateLegacyAccounts(raw);

  const activeAccountId =

    (typeof raw.activeAccountId === 'string' ? raw.activeAccountId : '') ||

    accounts[0]?.id ||

    '';

  return {

    theme: (raw.theme as Theme) ?? DEFAULT_SETTINGS.theme,

    enableDiscordRpc:

      (raw.enableDiscordRpc as boolean) ?? DEFAULT_SETTINGS.enableDiscordRpc,

    defaultPreset:

      (raw.defaultPreset as DefaultPreset) ?? DEFAULT_SETTINGS.defaultPreset,

    offlineUsername:

      (raw.offlineUsername as string) ?? DEFAULT_SETTINGS.offlineUsername,

    microsoftLinked:

      (raw.microsoftLinked as boolean) ?? DEFAULT_SETTINGS.microsoftLinked,

    accounts,

    activeAccountId,

    onboardingComplete:

      (raw.onboardingComplete as boolean) ?? DEFAULT_SETTINGS.onboardingComplete,

    presetApplied:

      (raw.presetApplied as boolean) ?? DEFAULT_SETTINGS.presetApplied,

    javaPath: (raw.javaPath as string) ?? DEFAULT_SETTINGS.javaPath,

    memoryMb: (raw.memoryMb as number) ?? DEFAULT_SETTINGS.memoryMb,

    closeOnLaunch:

      (raw.closeOnLaunch as boolean) ?? DEFAULT_SETTINGS.closeOnLaunch,

    showSnapshots:

      (raw.showSnapshots as boolean) ?? DEFAULT_SETTINGS.showSnapshots,

    lastSelectedInstanceId:

      (raw.lastSelectedInstanceId as string) ?? DEFAULT_SETTINGS.lastSelectedInstanceId,

  };

}

function toBackend(settings: AppSettings): Record<string, unknown> {

  return { ...settings };

}

const settingsStore = writable<AppSettings>(DEFAULT_SETTINGS);

let persistTimer: ReturnType<typeof setTimeout> | null = null;

function schedulePersist(settings: AppSettings) {

  if (persistTimer) clearTimeout(persistTimer);

  persistTimer = setTimeout(() => {

    saveSettings(toBackend(settings)).catch(console.error);

  }, 300);

}

function createSettingsStore() {

  const { subscribe, set, update } = settingsStore;

  return {

    subscribe,

    set,

    update,

    setTheme(theme: Theme) {

      update((s) => {

        applyTheme(theme);

        const next = { ...s, theme };

        schedulePersist(next);

        return next;

      });

    },

    patch(partial: Partial<AppSettings>) {

      update((s) => {

        const next = { ...s, ...partial };

        if (partial.theme) applyTheme(partial.theme);

        schedulePersist(next);

        return next;

      });

    },

    completeOnboarding(partial: Partial<AppSettings>) {

      update((s) => {

        const next = { ...s, ...partial, onboardingComplete: true };

        applyTheme(next.theme);

        schedulePersist(next);

        return next;

      });

    },

    async reloadFromBackend() {

      const raw = await getSettings();

      if (Object.keys(raw).length === 0) return;

      const loaded = fromBackend(raw);

      set(loaded);

      applyTheme(loaded.theme);

    },

    async flush() {

      if (persistTimer) {

        clearTimeout(persistTimer);

        persistTimer = null;

      }

      await saveSettings(toBackend(get(settingsStore)));

    },

    async init() {

      try {

        const raw = await getSettings();

        if (Object.keys(raw).length > 0) {

          const loaded = fromBackend(raw);

          set(loaded);

          applyTheme(loaded.theme);

          const active = loaded.accounts.find((a) => a.id === loaded.activeAccountId);

          if (active && !active.microsoftLinked) {

            await loginOffline(active.username).catch(console.error);

          } else if (loaded.offlineUsername && !loaded.microsoftLinked && !active) {

            await loginOffline(loaded.offlineUsername).catch(console.error);

          }

        } else {

          applyTheme(get(settingsStore).theme);

        }

      } catch {

        applyTheme(get(settingsStore).theme);

      }

      const mq = window.matchMedia('(prefers-color-scheme: dark)');

      mq.addEventListener('change', () => {

        const s = get(settingsStore);

        if (s.theme === 'system') applyTheme('system');

      });

    },

  };

}

export const settings = createSettingsStore();

export const resolvedTheme = derived(settingsStore, ($s) => {

  if ($s.theme === 'system') {

    return window.matchMedia('(prefers-color-scheme: dark)').matches

      ? 'dark'

      : 'light';

  }

  return $s.theme;

});

export const activeAccount = derived(settingsStore, ($s) =>

  $s.accounts.find((a) => a.id === $s.activeAccountId) ?? null

);

export const isLoggedIn = derived(settingsStore, ($s) => {

  if ($s.activeAccountId && $s.accounts.some((a) => a.id === $s.activeAccountId)) {

    return true;

  }

  return $s.offlineUsername.length > 0 || $s.microsoftLinked;

});

export function otherAccounts(all: AppSettings): SavedAccount[] {

  return all.accounts.filter((a) => a.id !== all.activeAccountId);

}
