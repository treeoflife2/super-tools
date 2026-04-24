<script lang="ts">
  import { onMount } from 'svelte';
  import { githubConnected, syncing, lastSyncedAt } from '$lib/stores/github';
  import { updateAvailable, showWhatsNewModal } from '$lib/utils/updater';

  let appVersion = $state('');
  onMount(async () => {
    try {
      const { getVersion } = await import('@tauri-apps/api/app');
      appVersion = await getVersion();
    } catch {
      appVersion = '';
    }
  });

  let syncLabel = $derived(
    $syncing ? 'Syncing...' :
    $githubConnected ? ($lastSyncedAt ? 'Gist synced' : 'Connected') :
    'Not connected'
  );

  let syncColor = $derived(
    $syncing ? 'var(--warn)' :
    $githubConnected ? 'var(--ok)' :
    'var(--t3)'
  );

  function openUpdateModal() {
    showWhatsNewModal.set(true);
  }
</script>

<footer class="statusbar glass-surface">
  <div class="sr">
    <div class="si">
      <span class="sled" style="background:{syncColor}"></span>
      <svg style="width:10px;height:10px;stroke:var(--t3);fill:none;stroke-width:1.7;stroke-linecap:round" viewBox="0 0 24 24"><path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 00-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0020 4.77 5.07 5.07 0 0019.91 1S18.73.65 16 2.48a13.38 13.38 0 00-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 005 4.77a5.44 5.44 0 00-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 009 18.13V22"/></svg>
      <span>{syncLabel}</span>
    </div>
    {#if $updateAvailable}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="si update-hint" onclick={openUpdateModal}>
        <span class="sled update-dot"></span>
        <span>Update available · v{$updateAvailable.version}</span>
      </div>
    {/if}
    {#if appVersion}<div class="si">Qorix v{appVersion}</div>{/if}
  </div>
</footer>

<style>
  .statusbar {
    height: 24px;
    background: var(--s);
    border-top: 1px solid var(--b1);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 16px;
    flex-shrink: 0;
  }
  .si {
    font-size: 10px;
    color: var(--t3);
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: var(--mono);
  }
  .sled {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .sr {
    margin-left: auto;
    display: flex;
    gap: 16px;
  }
  .update-hint {
    cursor: default;
    color: var(--acc, #7c5cf8);
    transition: opacity 0.12s;
  }
  .update-hint:hover {
    opacity: 0.8;
  }
  .update-dot {
    background: var(--acc, #7c5cf8) !important;
    animation: updatePulse 2s ease-in-out infinite;
  }
  @keyframes updatePulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
