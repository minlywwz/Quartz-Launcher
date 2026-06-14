import { writable } from 'svelte/store';
import type { LauncherPanel } from './launcher';

export type AppView = 'onboarding' | 'login' | 'main';

export const loginCancellable = writable(false);
export const loginReturnPanel = writable<LauncherPanel>('home');

function createViewStore() {
  const { subscribe, set } = writable<AppView>('onboarding');

  function clearLoginSwitch() {
    loginCancellable.set(false);
  }

  return {
    subscribe,
    set,
    goTo: (v: AppView) => set(v),

    goToMain: () => {
      clearLoginSwitch();
      set('main');
    },

    goToLogin: () => {
      clearLoginSwitch();
      set('login');
    },

    goToLoginForSwitch: (returnPanel: LauncherPanel = 'home') => {
      loginReturnPanel.set(returnPanel);
      loginCancellable.set(true);
      set('login');
    },

    cancelLogin: () => {
      clearLoginSwitch();
      set('main');
    },

    goToOnboarding: () => set('onboarding'),
  };
}

export const currentView = createViewStore();
