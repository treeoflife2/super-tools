<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    loadCanvas,
    flushViewportNow,
    flushDirtyTilesNow,
    setActiveWorkspace,
    viewport,
  } from '$lib/modes/canvas/stores/canvasStore';
  import { canvasGetViewport } from '$lib/modes/canvas/commands';
  import CanvasViewport from './CanvasViewport.svelte';

  // Phase 2 stub: hardcoded workspace id so the surface mounts. Phase 4
  // wires this to the real active-workspace store.
  const ACTIVE_WORKSPACE_ID = '__phase2_stub__';

  onMount(async () => {
    setActiveWorkspace(ACTIVE_WORKSPACE_ID);
    const v = await canvasGetViewport(ACTIVE_WORKSPACE_ID);
    viewport.set({ offsetX: v.offsetX, offsetY: v.offsetY, zoom: v.zoom });
    // Phase 2 passes empty open-tab list — Phase 3 fills via adapters.
    await loadCanvas(ACTIVE_WORKSPACE_ID, []);
  });

  onDestroy(() => {
    // Svelte does not await async onDestroy callbacks, so we fire-and-forget
    // both flushes. Phase 2 has no tile data yet; Phase 3 may need a different
    // strategy (synchronous local-storage write, beforeunload listener) to
    // avoid losing in-flight dirty geometry on rapid mode switches.
    void flushViewportNow();
    void flushDirtyTilesNow();
  });
</script>

<div class="cv-panel">
  <CanvasViewport />
</div>

<style>
  .cv-panel {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
  }
</style>
