import { get } from 'svelte/store';
import {
  tilesByTab,
  interactionState,
  markTileDirty,
  viewport,
} from '$lib/modes/canvas/stores/canvasStore';

type Direction = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

interface ResizableParams {
  tabId: string;
  dir: Direction;
}

const MIN_WIDTH = 240;
const MIN_HEIGHT = 160;

export function resizable(node: HTMLElement, params: ResizableParams) {
  let { tabId, dir } = params;
  let resizing = false;
  let pointerId: number | null = null;
  let startCanvasX = 0;
  let startCanvasY = 0;
  let startTile = { x: 0, y: 0, width: 0, height: 0 };

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const tile = get(tilesByTab).get(tabId);
    if (!tile) return;
    e.stopPropagation();
    e.preventDefault();
    resizing = true;
    pointerId = e.pointerId;
    node.setPointerCapture(e.pointerId);
    const v = get(viewport);
    startCanvasX = (e.clientX - v.offsetX) / v.zoom;
    startCanvasY = (e.clientY - v.offsetY) / v.zoom;
    startTile = { x: tile.x, y: tile.y, width: tile.width, height: tile.height };
    interactionState.set('resizing');
    document.body.classList.add('cv-interacting');
  }

  function onPointerMove(e: PointerEvent) {
    if (!resizing || e.pointerId !== pointerId) return;
    const v = get(viewport);
    const curCanvasX = (e.clientX - v.offsetX) / v.zoom;
    const curCanvasY = (e.clientY - v.offsetY) / v.zoom;
    const dx = curCanvasX - startCanvasX;
    const dy = curCanvasY - startCanvasY;

    let { x, y, width, height } = startTile;

    if (dir.includes('e')) width = Math.max(MIN_WIDTH, startTile.width + dx);
    if (dir.includes('s')) height = Math.max(MIN_HEIGHT, startTile.height + dy);
    if (dir.includes('w')) {
      const newWidth = Math.max(MIN_WIDTH, startTile.width - dx);
      x = startTile.x + (startTile.width - newWidth);
      width = newWidth;
    }
    if (dir.includes('n')) {
      const newHeight = Math.max(MIN_HEIGHT, startTile.height - dy);
      y = startTile.y + (startTile.height - newHeight);
      height = newHeight;
    }

    tilesByTab.update((m) => {
      const next = new Map(m);
      const cur = next.get(tabId);
      if (cur) next.set(tabId, { ...cur, x, y, width, height });
      return next;
    });
  }

  function endResize(e: PointerEvent) {
    if (!resizing || e.pointerId !== pointerId) return;
    resizing = false;
    pointerId = null;
    try {
      node.releasePointerCapture(e.pointerId);
    } catch {
      // Capture may already be released.
    }
    interactionState.set('idle');
    document.body.classList.remove('cv-interacting');
    markTileDirty(tabId);
  }

  node.addEventListener('pointerdown', onPointerDown);
  node.addEventListener('pointermove', onPointerMove);
  node.addEventListener('pointerup', endResize);
  node.addEventListener('pointercancel', endResize);

  return {
    update(p: ResizableParams) {
      tabId = p.tabId;
      dir = p.dir;
    },
    destroy() {
      node.removeEventListener('pointerdown', onPointerDown);
      node.removeEventListener('pointermove', onPointerMove);
      node.removeEventListener('pointerup', endResize);
      node.removeEventListener('pointercancel', endResize);
    },
  };
}
