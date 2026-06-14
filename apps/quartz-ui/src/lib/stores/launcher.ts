import { writable } from 'svelte/store';

export type LauncherPanel = 'home' | 'browse' | 'settings';

function createLauncherStore() {
  const { subscribe, set, update } = writable<LauncherPanel>('home');

  return {
    subscribe,
    goHome: () => set('home'),
    goBrowse: () => set('browse'),
    goSettings: () => set('settings'),
    back: () => set('home'),
  };
}

export const launcherPanel = createLauncherStore();
