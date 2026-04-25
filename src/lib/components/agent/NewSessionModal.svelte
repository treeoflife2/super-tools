<script lang="ts">
  import Modal from '$lib/components/shared/Modal.svelte';
  import { agentCreateSession, agentDiscoverSessions, agentListContexts, agentAttachContext, agentUpdateSessionId } from '$lib/commands/agent';
  import type { AgentContext, DiscoveredSession } from '$lib/types/agent';
  import { loadAgentSessions } from '$lib/stores/agent';
  import { showToast } from '$lib/components/shared/toast';

  let { show = $bindable(false) } = $props();

  let title = $state('');
  let purpose = $state('Development');
  let projectPath = $state('');
  let skipPermissions = $state(false);
  let customPrompt = $state('');
  let gitName = $state('');
  let gitEmail = $state('');
  let loading = $state(false);

  // Resume existing session
  let resumeEnabled = $state(false);
  let discoveredSessions = $state<DiscoveredSession[]>([]);
  let selectedSessionId = $state('');

  // Context attachment
  let availableContexts = $state<AgentContext[]>([]);
  let attachedContextIds = $state<string[]>([]);
  let showContextDropdown = $state(false);
  let showAdvanced = $state(false);
  let contextsLoaded = $state(false);

  const purposes = [
    { name: 'Brainstorming', color: '#d2a8ff' },
    { name: 'Development', color: '#3fb950' },
    { name: 'Code Review', color: '#58a6ff' },
    { name: 'PR Review', color: '#d29922' },
    { name: 'Debugging', color: '#f85149' },
    { name: 'Custom', color: '#8b949e' },
  ];

  async function pickFolder() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      projectPath = selected as string;
      if (!title) title = (selected as string).split('/').filter(Boolean).pop() || '';
      loadDiscoveredSessions(selected as string);
    }
  }

  async function loadDiscoveredSessions(path: string) {
    try {
      discoveredSessions = await agentDiscoverSessions(path);
      selectedSessionId = '';
    } catch (_) {
      discoveredSessions = [];
    }
  }

  async function loadContexts() {
    try {
      availableContexts = await agentListContexts();
    } catch (_) {
      availableContexts = [];
    }
  }

  async function handleCreate() {
    if (!title.trim() || !projectPath.trim()) return;
    loading = true;
    try {
      const session = await agentCreateSession({
        title: title.trim(),
        purpose,
        projectPath: projectPath.trim(),
        skipPermissions: skipPermissions || undefined,
        customPrompt: purpose === 'Custom' && customPrompt.trim() ? customPrompt.trim() : undefined,
        gitName: gitName.trim() || undefined,
        gitEmail: gitEmail.trim() || undefined,
      });

      // Attach resumed Claude session ID if selected
      if (resumeEnabled && selectedSessionId) {
        await agentUpdateSessionId(session.id, selectedSessionId);
      }

      // Attach selected contexts
      for (const contextId of attachedContextIds) {
        await agentAttachContext(session.id, contextId);
      }

      await loadAgentSessions();
      show = false;
      resetForm();
    } catch (e: any) {
      showToast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    title = '';
    purpose = 'Development';
    projectPath = '';
    skipPermissions = false;
    customPrompt = '';
    gitName = '';
    gitEmail = '';
    resumeEnabled = false;
    discoveredSessions = [];
    selectedSessionId = '';
    attachedContextIds = [];
    showContextDropdown = false;
    showAdvanced = false;
    contextsLoaded = false;
  }
</script>

<Modal bind:show title="New Agent Session" width="520px">
  <div class="ns-form">
    <label class="ns-field">
      <span class="ns-label">Project Path</span>
      <div class="ns-path-row">
        <input
          class="ns-input ns-path-input"
          type="text"
          bind:value={projectPath}
          placeholder="/path/to/project"
          onblur={() => { if (projectPath.trim()) loadDiscoveredSessions(projectPath.trim()); }}
        />
        <button class="ns-btn outline" onclick={pickFolder}>Browse</button>
      </div>
    </label>

    <label class="ns-field">
      <span class="ns-label">Title</span>
      <input class="ns-input" type="text" bind:value={title} placeholder="Session title" />
    </label>

    <div class="ns-field">
      <span class="ns-label">Purpose</span>
      <div class="ns-chips">
        {#each purposes as p}
          <button
            class="ns-chip"
            class:active={purpose === p.name}
            style="--chip-color: {p.color}"
            onclick={() => purpose = p.name}
          >
            <span class="ns-chip-dot" style="background: {p.color}"></span>
            {p.name}
          </button>
        {/each}
      </div>
    </div>

    {#if discoveredSessions.length > 0 && purpose !== 'Custom'}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="ns-session-hint" onclick={() => { purpose = 'Custom'; resumeEnabled = true; }}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--acc)" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
        <span>{discoveredSessions.length} previous session{discoveredSessions.length > 1 ? 's' : ''} found — <strong>resume via Custom</strong></span>
      </div>
    {/if}

    {#if purpose === 'Custom'}
      {#if discoveredSessions.length > 0}
        <label class="ns-check">
          <input type="checkbox" bind:checked={resumeEnabled} />
          <span>Resume existing session</span>
        </label>
        {#if resumeEnabled}
          <label class="ns-field">
            <span class="ns-label">Select Session</span>
            <select class="ns-select" bind:value={selectedSessionId}>
              <option value="">Start fresh</option>
              {#each discoveredSessions as s}
                <option value={s.sessionId}>{s.preview || s.sessionId.slice(0, 8)} — {new Date(s.modifiedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</option>
              {/each}
            </select>
          </label>
        {/if}
      {/if}
      <label class="ns-field">
        <span class="ns-label">Custom Prompt <span class="ns-optional">(optional)</span></span>
        <textarea
          class="ns-textarea"
          bind:value={customPrompt}
          placeholder="Describe the purpose of this session..."
          rows="3"
        ></textarea>
      </label>
    {/if}

    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="ns-advanced-toggle" onclick={() => { showAdvanced = !showAdvanced; if (showAdvanced && !contextsLoaded) { loadContexts(); contextsLoaded = true; } }}>
      <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="transform:rotate({showAdvanced ? 90 : 0}deg);transition:transform 0.15s"><path d="M6 4l4 4-4 4"/></svg>
      <span>Advanced Options</span>
    </div>

    {#if showAdvanced}
    <div class="ns-advanced-body">
      <label class="ns-check">
        <input type="checkbox" bind:checked={skipPermissions} />
        <span>Skip permission prompts</span>
      </label>

      <div class="ns-field">
        <span class="ns-label">Git Identity (optional)</span>
        <div class="ns-row">
          <input class="ns-input" style="flex:1" type="text" bind:value={gitName} placeholder="Name" />
          <input class="ns-input" style="flex:1" type="text" bind:value={gitEmail} placeholder="Email" />
        </div>
      </div>

      <div class="ns-field">
        <span class="ns-label">Attach Contexts</span>
      </div>
      <div class="ns-ctx-area">
        {#if attachedContextIds.length > 0}
          <div class="ns-ctx-chips">
            {#each attachedContextIds as cid}
              {@const ctx = availableContexts.find(c => c.id === cid)}
              {#if ctx}
                <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                <span class="ns-ctx-chip">
                  {ctx.name}
                  <span class="ns-ctx-chip-x" onclick={() => { attachedContextIds = attachedContextIds.filter(id => id !== cid); }}>x</span>
                </span>
              {/if}
            {/each}
          </div>
        {/if}
        <div class="ns-ctx-add-wrap">
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <button class="ns-ctx-add-btn" onclick={(e) => { e.stopPropagation(); showContextDropdown = !showContextDropdown; }}>
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor"><path d="M7.75 2a.75.75 0 01.75.75V7h4.25a.75.75 0 010 1.5H8.5v4.25a.75.75 0 01-1.5 0V8.5H2.75a.75.75 0 010-1.5H7V2.75A.75.75 0 017.75 2z"/></svg>
            Add Context
          </button>
          {#if showContextDropdown}
            <div class="ns-ctx-dropdown">
              {#each availableContexts.filter(c => !attachedContextIds.includes(c.id)) as ctx}
                <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                <div class="ns-ctx-dropdown-item" onclick={() => { attachedContextIds = [...attachedContextIds, ctx.id]; showContextDropdown = false; }}>
                  <span class="ns-ctx-dropdown-name">{ctx.name}</span>
                  <span class="ns-ctx-dropdown-preview">{ctx.content.slice(0, 60)}</span>
                </div>
              {:else}
                <div class="ns-ctx-dropdown-empty">No contexts saved yet</div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
    {/if}

    <div class="ns-actions">
      <button class="ns-btn outline" onclick={() => show = false}>Cancel</button>
      <button
        class="ns-btn primary"
        onclick={handleCreate}
        disabled={!title.trim() || !projectPath.trim() || loading}
      >
        {loading ? 'Creating...' : 'Create Session'}
      </button>
    </div>
  </div>
</Modal>

<style>
  .ns-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .ns-advanced-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--t3);
    font-family: var(--ui);
    cursor: default;
    padding: 4px 0;
    user-select: none;
  }
  .ns-advanced-toggle:hover {
    color: var(--t2);
  }
  .ns-advanced-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--b1);
    border-radius: 8px;
    background: color-mix(in srgb, var(--n) 50%, transparent);
  }
  .ns-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ns-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--t2);
    font-family: var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .ns-input {
    height: 32px;
    background: var(--e);
    border: 1px solid var(--b1);
    border-radius: 6px;
    padding: 0 10px;
    font-size: 12.5px;
    font-family: var(--mono);
    color: var(--t1);
    outline: none;
    transition: border-color 0.15s;
  }
  .ns-input:focus {
    border-color: var(--acc);
  }
  .ns-input::placeholder {
    color: var(--t3);
  }
  .ns-textarea {
    background: var(--e);
    border: 1px solid var(--b1);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12.5px;
    font-family: var(--mono);
    color: var(--t1);
    outline: none;
    resize: vertical;
    transition: border-color 0.15s;
  }
  .ns-textarea:focus {
    border-color: var(--acc);
  }
  .ns-textarea::placeholder {
    color: var(--t3);
  }
  .ns-path-row {
    display: flex;
    gap: 8px;
  }
  .ns-path-input {
    flex: 1;
  }
  .ns-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }
  .ns-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 14px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11.5px;
    font-family: var(--ui);
    cursor: default;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
  }
  .ns-chip:hover {
    border-color: var(--b2);
    color: var(--t1);
  }
  .ns-chip.active {
    border-color: var(--chip-color);
    color: var(--t1);
    background: color-mix(in srgb, var(--chip-color) 12%, transparent);
  }
  .ns-chip-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ns-check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--t2);
    font-family: var(--ui);
    cursor: default;
  }
  .ns-check input {
    accent-color: var(--acc);
  }
  .ns-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .ns-section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--t3);
    font-family: var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .ns-row {
    display: flex;
    gap: 10px;
  }
  .ns-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--b1);
  }
  .ns-btn {
    height: 34px;
    padding: 0 20px;
    border-radius: 8px;
    font-size: 12px;
    font-family: var(--ui);
    cursor: default;
    transition: opacity 0.12s, border-color 0.12s, color 0.12s;
  }
  .ns-btn.outline {
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
  }
  .ns-btn.outline:hover:not(:disabled) {
    border-color: var(--b2);
    color: var(--t1);
  }
  .ns-btn.outline:disabled {
    opacity: 0.5;
  }
  .ns-btn.primary {
    border: none;
    background: var(--acc);
    color: #fff;
    font-weight: 600;
  }
  .ns-btn.primary:hover:not(:disabled) {
    opacity: 0.85;
  }
  .ns-btn.primary:disabled {
    opacity: 0.4;
  }
  .ns-select {
    height: 32px;
    background: var(--e);
    border: 1px solid var(--b1);
    border-radius: 6px;
    padding: 0 10px;
    font-size: 12.5px;
    font-family: var(--mono);
    color: var(--t1);
    outline: none;
    appearance: none;
    cursor: default;
    transition: border-color 0.15s;
  }
  .ns-select:focus {
    border-color: var(--acc);
  }
  .ns-select option {
    background: var(--n);
    color: var(--t1);
  }
  .ns-optional {
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: var(--t3);
  }
  .ns-session-hint {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--acc) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--acc) 20%, transparent);
    cursor: default;
  }
  .ns-session-hint span {
    font-size: 11px;
    color: var(--t2);
    line-height: 1.4;
  }
  .ns-session-hint strong {
    color: var(--acc);
    cursor: default;
  }
  .ns-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .ns-ctx-area {
    display: flex;
    flex-direction: column;
    gap: 8px;
    animation: ctxFadeIn 0.12s ease;
  }
  @keyframes ctxFadeIn { from { opacity: 0; } to { opacity: 1; } }
  .ns-ctx-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .ns-ctx-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 6px 3px 10px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--acc) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--acc) 25%, transparent);
    color: var(--acc);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--ui);
  }
  .ns-ctx-chip-x {
    cursor: default;
    font-size: 13px;
    line-height: 1;
    opacity: 0.6;
    transition: opacity 0.1s;
    display: flex;
    align-items: center;
  }
  .ns-ctx-chip-x:hover {
    opacity: 1;
  }
  .ns-ctx-add-wrap {
    position: relative;
  }
  .ns-ctx-add-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 4px;
    border: 1px dashed var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11px;
    font-family: var(--ui);
    cursor: default;
    transition: border-color 0.1s, color 0.1s;
  }
  .ns-ctx-add-btn:hover {
    border-color: var(--acc);
    color: var(--acc);
  }
  .ns-ctx-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    width: 260px;
    background: var(--n);
    border: 1px solid var(--b1);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 100;
    max-height: 180px;
    overflow-y: auto;
    padding: 4px;
  }
  .ns-ctx-dropdown-item {
    padding: 6px 10px;
    border-radius: 4px;
    cursor: default;
    transition: background 0.1s;
  }
  .ns-ctx-dropdown-item:hover {
    background: rgba(255,255,255,0.06);
  }
  .ns-ctx-dropdown-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--t1);
    display: block;
    font-family: var(--ui);
  }
  .ns-ctx-dropdown-preview {
    font-size: 10px;
    color: var(--t3);
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }
  .ns-ctx-dropdown-empty {
    padding: 10px;
    text-align: center;
    font-size: 11px;
    color: var(--t3);
    font-family: var(--ui);
  }
</style>
