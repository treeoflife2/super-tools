import { writable, derived, get } from 'svelte/store';
import type { CanvasTile, TabRef } from '$lib/modes/canvas/commands';
import {
  canvasResolveTiles,
  canvasSetViewport,
  canvasUpsertTilesBatch,
} from '$lib/modes/canvas/commands';

export interface ViewportState {
  offsetX: number;
  offsetY: number;
  zoom: number;
}

export type InteractionState = 'idle' | 'panning' | 'zooming' | 'dragging' | 'resizing';

export const ZOOM_MIN = 0.25;
export const ZOOM_MAX = 3.0;
export const ZOOM_DEFAULT = 1.0;
export const VIEWPORT_FLUSH_DEBOUNCE_MS = 1000;
export const TILE_FLUSH_DEBOUNCE_MS = 500;

// Tile map keyed by tabId. Callers MUST treat the Map as immutable:
// `tilesByTab.update(m => { const next = new Map(m); next.set(id, tile); return next; })`.
// Mutating in place skips subscriber notifications. Phase 3 will introduce
// typed setter helpers; until then, follow the copy-on-mutate pattern.
export const tilesByTab = writable<Map<string, CanvasTile>>(new Map());
export const viewport = writable<ViewportState>({ offsetX: 0, offsetY: 0, zoom: ZOOM_DEFAULT });
export const interactionState = writable<InteractionState>('idle');
export const focusedTabId = writable<string | null>(null);

// Debounced viewport flush — single timer.
let viewportFlushTimer: ReturnType<typeof setTimeout> | null = null;
// Singleton: only one canvas workspace is active at a time (Clauge is single-window).
let currentWorkspaceId: string | null = null;

export function setActiveWorkspace(workspaceId: string | null): void {
  currentWorkspaceId = workspaceId;
}

export function flushViewportSoon(): void {
  if (!currentWorkspaceId) return;
  if (viewportFlushTimer) clearTimeout(viewportFlushTimer);
  viewportFlushTimer = setTimeout(() => {
    viewportFlushTimer = null;
    void flushViewportNow();
  }, VIEWPORT_FLUSH_DEBOUNCE_MS);
}

export async function flushViewportNow(): Promise<void> {
  if (!currentWorkspaceId) return;
  if (viewportFlushTimer) {
    clearTimeout(viewportFlushTimer);
    viewportFlushTimer = null;
  }
  const v = get(viewport);
  const focused = get(focusedTabId);
  await canvasSetViewport(currentWorkspaceId, v.offsetX, v.offsetY, v.zoom, focused);
}

// Replace store contents from backend. Phase 2 passes an empty open-tab list
// so backend creates no tiles. Phase 3 wires in real adapters.
export async function loadCanvas(workspaceId: string, openTabRefs: TabRef[]): Promise<void> {
  setActiveWorkspace(workspaceId);
  const tiles = await canvasResolveTiles(workspaceId, openTabRefs);
  const map = new Map<string, CanvasTile>();
  for (const t of tiles) map.set(t.tabId, t);
  tilesByTab.set(map);
}

// Dirty-tile tracking for Phase 3's geometry flush.
const dirtyTiles = new Set<string>();
let tileFlushTimer: ReturnType<typeof setTimeout> | null = null;

export function markTileDirty(tabId: string): void {
  dirtyTiles.add(tabId);
  if (tileFlushTimer) clearTimeout(tileFlushTimer);
  tileFlushTimer = setTimeout(() => {
    tileFlushTimer = null;
    void flushDirtyTilesNow();
  }, TILE_FLUSH_DEBOUNCE_MS);
}

export async function flushDirtyTilesNow(): Promise<void> {
  if (!currentWorkspaceId || dirtyTiles.size === 0) return;
  if (tileFlushTimer) {
    clearTimeout(tileFlushTimer);
    tileFlushTimer = null;
  }
  const map = get(tilesByTab);
  const snapshot = [...dirtyTiles];
  const updates = snapshot
    .map((id) => map.get(id))
    .filter((t): t is CanvasTile => !!t)
    .map((t) => ({
      workspaceId: t.workspaceId,
      tabId: t.tabId,
      x: t.x,
      y: t.y,
      width: t.width,
      height: t.height,
      zOrder: t.zOrder,
    }));
  dirtyTiles.clear();
  if (updates.length === 0) return;
  try {
    await canvasUpsertTilesBatch(updates);
  } catch (err) {
    // Re-queue snapshot ids so the next flush retries them.
    for (const id of snapshot) dirtyTiles.add(id);
    throw err;
  }
}

// Z-ordered list for rendering — Phase 3 consumes this.
export const tilesSortedByZ = derived(tilesByTab, ($map) =>
  [...$map.values()].sort((a, b) => a.zOrder - b.zOrder),
);
