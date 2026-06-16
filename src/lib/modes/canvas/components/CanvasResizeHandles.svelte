<script lang="ts">
  import type { CanvasTile as Tile } from '$lib/modes/canvas/commands';
  import { resizable } from '$lib/modes/canvas/actions/resizable';

  let { tile }: { tile: Tile } = $props();

  const handles: { dir: 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'; cursor: string }[] = [
    { dir: 'n', cursor: 'ns-resize' },
    { dir: 's', cursor: 'ns-resize' },
    { dir: 'e', cursor: 'ew-resize' },
    { dir: 'w', cursor: 'ew-resize' },
    { dir: 'ne', cursor: 'nesw-resize' },
    { dir: 'nw', cursor: 'nwse-resize' },
    { dir: 'se', cursor: 'nwse-resize' },
    { dir: 'sw', cursor: 'nesw-resize' },
  ];
</script>

{#each handles as h (h.dir)}
  <div
    class="cv-resize-handle cv-rh-{h.dir}"
    style="cursor: {h.cursor};"
    use:resizable={{ tabId: tile.tabId, dir: h.dir }}
  ></div>
{/each}

<style>
  .cv-resize-handle {
    position: absolute;
    z-index: 1;
  }
  .cv-rh-n {
    top: -3px;
    left: 8px;
    right: 8px;
    height: 6px;
  }
  .cv-rh-s {
    bottom: -3px;
    left: 8px;
    right: 8px;
    height: 6px;
  }
  .cv-rh-e {
    top: 8px;
    bottom: 8px;
    right: -3px;
    width: 6px;
  }
  .cv-rh-w {
    top: 8px;
    bottom: 8px;
    left: -3px;
    width: 6px;
  }
  .cv-rh-ne {
    top: -3px;
    right: -3px;
    width: 12px;
    height: 12px;
  }
  .cv-rh-nw {
    top: -3px;
    left: -3px;
    width: 12px;
    height: 12px;
  }
  .cv-rh-se {
    bottom: -3px;
    right: -3px;
    width: 12px;
    height: 12px;
  }
  .cv-rh-sw {
    bottom: -3px;
    left: -3px;
    width: 12px;
    height: 12px;
  }
</style>
