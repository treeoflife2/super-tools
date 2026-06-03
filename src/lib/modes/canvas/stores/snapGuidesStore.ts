import { writable } from 'svelte/store';

export interface GuideLine {
  axis: 'x' | 'y';
  position: number; // canvas-space coordinate
}

export const snapGuides = writable<GuideLine[]>([]);
