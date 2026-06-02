<script lang="ts">
  import type { CanvasTile as Tile } from '$lib/modes/canvas/commands';
  import { canvasAdapterRegistry } from '$lib/modes/canvas/adapter-registry';
  import { draggable } from '$lib/modes/canvas/actions/draggable';

  let { tile }: { tile: Tile } = $props();

  const adapter = $derived(canvasAdapterRegistry.get(tile.tabKind));
  const meta = $derived(adapter?.getMeta(tile.tabId) ?? { title: tile.tabId });

  function onDoubleClick() {
    adapter?.openInHomeMode(tile.tabId);
  }

  function onClose(e: MouseEvent) {
    e.stopPropagation();
    void adapter?.closeTab?.(tile.tabId);
  }
</script>

<div
  class="cv-tile-titlebar"
  use:draggable={{ tabId: tile.tabId }}
  ondblclick={onDoubleClick}
>
  {#if meta.statusDot}
    <span class="cv-tile-dot" style="background: {meta.statusDot};"></span>
  {/if}
  <span class="cv-tile-title">{meta.title}{meta.dirty ? ' •' : ''}</span>
  <button class="cv-tile-close" onclick={onClose} title="Close" aria-label="Close tile">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
      <line x1="2" y1="2" x2="12" y2="12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      <line x1="12" y1="2" x2="2" y2="12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>
  </button>
</div>

<style>
  .cv-tile-titlebar {
    height: 26px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    background: var(--cv-tile-titlebar-bg, var(--surface-card));
    border-bottom: 1px solid var(--cv-tile-border, var(--b1));
    cursor: grab;
    user-select: none;
    font-size: 12px;
    color: var(--cv-tile-title-fg, var(--t1));
  }
  .cv-tile-titlebar:active {
    cursor: grabbing;
  }
  .cv-tile-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .cv-tile-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cv-tile-close {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    padding: 0;
    margin-left: 4px;
    background: transparent;
    border: none;
    border-radius: 3px;
    color: var(--t3);
    cursor: pointer;
    line-height: 1;
  }
  .cv-tile-close:hover {
    color: var(--t1);
    background: var(--surface-hover);
  }
</style>
