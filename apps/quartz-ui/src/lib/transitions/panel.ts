import type { TransitionConfig } from 'svelte/transition';
import type { LauncherPanel } from '$lib/stores/launcher';

export const PANEL_TRANSITION_MS = 250;

const PANEL_ORDER: Record<LauncherPanel, number> = {
  home: 0,
  browse: 1,
  settings: 2,
};

const TRAVEL_IN = 118;
const TRAVEL_OUT = 62;

export function panelDirection(from: LauncherPanel, to: LauncherPanel): number {
  if (from === to) return 1;
  return PANEL_ORDER[to] > PANEL_ORDER[from] ? 1 : -1;
}

function easeFlight(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

interface BlurSlideParams {
  direction?: number;
  duration?: number;
}

export function blurSlideIn(
  node: Element,
  { direction = 1, duration = PANEL_TRANSITION_MS }: BlurSlideParams = {}
): TransitionConfig {
  return {
    duration,
    css: (t, u) => {
      const e = easeFlight(t);
      const x = u * direction * TRAVEL_IN;
      const scale = 0.82 + e * 0.18;
      const blur = u * 10;
      const opacity = Math.min(1, e * 1.15);
      return (
        `opacity:${opacity};` +
        `transform:translate3d(${x}%,0,0) scale(${scale});` +
        `filter:blur(${blur}px);`
      );
    },
  };
}

export function blurSlideOut(
  node: Element,
  { direction = 1, duration = PANEL_TRANSITION_MS }: BlurSlideParams = {}
): TransitionConfig {
  return {
    duration,
    css: (t, u) => {
      const e = easeFlight(t);
      const x = -u * direction * TRAVEL_OUT;
      const scale = 0.86 + e * 0.14;
      const blur = u * 8;
      return (
        `opacity:${e};` +
        `transform:translate3d(${x}%,0,0) scale(${scale});` +
        `filter:blur(${blur}px);`
      );
    },
  };
}
