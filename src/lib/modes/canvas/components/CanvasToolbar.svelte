<script lang="ts">
  import {
    viewport,
    ZOOM_MIN,
    ZOOM_MAX,
    ZOOM_DEFAULT,
    flushViewportSoon,
  } from '$lib/modes/canvas/stores/canvasStore';

  function setZoom(z: number) {
    viewport.update((v) => ({ ...v, zoom: Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z)) }));
    flushViewportSoon();
  }

  function reset() {
    viewport.set({ offsetX: 0, offsetY: 0, zoom: ZOOM_DEFAULT });
    flushViewportSoon();
  }

  const zoomPct = $derived(Math.round($viewport.zoom * 100));
</script>

<div class="cv-toolbar">
  <button class="cv-tb-btn" onclick={() => setZoom($viewport.zoom / 1.2)} aria-label="Zoom out">−</button>
  <button class="cv-tb-btn cv-tb-pct" onclick={reset} title="Reset view" aria-label="Reset view">{zoomPct}%</button>
  <button class="cv-tb-btn" onclick={() => setZoom($viewport.zoom * 1.2)} aria-label="Zoom in">+</button>
</div>

<style>
  .cv-toolbar {
    position: absolute;
    bottom: 12px;
    right: 12px;
    display: flex;
    gap: 4px;
    padding: 4px;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 8px;
    backdrop-filter: blur(8px);
  }
  .cv-tb-btn {
    height: 28px;
    min-width: 28px;
    padding: 0 8px;
    background: transparent;
    color: rgba(255, 255, 255, 0.85);
    border: 0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }
  .cv-tb-btn:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .cv-tb-pct {
    min-width: 56px;
    font-variant-numeric: tabular-nums;
  }
</style>
