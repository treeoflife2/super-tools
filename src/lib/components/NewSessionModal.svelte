<script>
  import { invoke } from "@tauri-apps/api/core";
  import { contextsStore } from "$lib/stores/contexts.svelte";

  let {
    show = $bindable(),
    profiles,
    activeProfile,
    onCreate,
  } = $props();

  // Modal-local state
  let modalPath = $state("");
  let modalTitle = $state("");
  let modalPurpose = $state("");
  let modalSkipPermissions = $state(false);
  let modalExistingSessions = $state([]);
  let modalSelectedSession = $state("");
  let modalCustomPrompt = $state("");
  let modalGitEnabled = $state(false);
  let modalGitName = $state("");
  let modalGitEmail = $state("");

  const purposes = [
    { label: "Brainstorming", color: "#d2a8ff" },
    { label: "Development", color: "#3fb950" },
    { label: "Code Review", color: "#58a6ff" },
    { label: "PR Review", color: "#d29922" },
    { label: "Debugging", color: "#f85149" },
    { label: "Custom", color: "#8b949e" },
  ];

  async function loadExistingSessions(path) {
    try {
      const sessions = await invoke("discover_sessions", { projectPath: path });
      // Filter out sessions already linked to a profile
      const linkedIds = new Set(profiles.filter(p => p.claudeSessionId).map(p => p.claudeSessionId));
      modalExistingSessions = sessions.filter(s => !linkedIds.has(s.sessionId));
      modalSelectedSession = "";
    } catch(_) { modalExistingSessions = []; }
  }

  async function browsePath() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false, title: "Select Project Folder" });
      if (selected) {
        modalPath = selected;
        if (!modalTitle) modalTitle = selected.split("/").filter(Boolean).pop() || "";
        loadExistingSessions(selected);
      }
    } catch(e) { /* ignore */ }
  }

  function resetState() {
    modalPath = "";
    modalTitle = "";
    modalPurpose = "";
    modalSkipPermissions = false;
    modalExistingSessions = [];
    modalSelectedSession = "";
    modalCustomPrompt = "";
    modalGitEnabled = false;
    modalGitName = "";
    modalGitEmail = "";
    contextsStore.modalContexts = [];
    contextsStore.modalContextEnabled = false;
    contextsStore.showContextDropdown = false;
  }

  function handleCancel() {
    show = false;
    resetState();
  }

  function handleCreate() {
    if (!modalPath || !modalTitle || !modalPurpose) return;
    onCreate({
      path: modalPath,
      title: modalTitle,
      purpose: modalPurpose,
      skipPermissions: modalSkipPermissions,
      customPrompt: modalPurpose === 'Custom' && modalCustomPrompt.trim() ? modalCustomPrompt.trim() : null,
      gitName: modalGitEnabled && modalGitName.trim() ? modalGitName.trim() : null,
      gitEmail: modalGitEnabled && modalGitEmail.trim() ? modalGitEmail.trim() : null,
      selectedSession: modalSelectedSession || null,
      contexts: contextsStore.modalContexts.length > 0 ? contextsStore.modalContexts : null,
    });
    show = false;
    resetState();
  }
</script>

{#if show}
<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div class="modal-backdrop">
  <div class="modal">
    <h2>New Session</h2>
    <label>Project Folder
      <div class="row">
        <input bind:value={modalPath} placeholder="/path/to/project" onblur={() => { if (modalPath.trim()) loadExistingSessions(modalPath); }} />
        <button onclick={browsePath}>Browse</button>
      </div>
    </label>
    <label>Title
      <input bind:value={modalTitle} placeholder="e.g. Auth Refactor" />
    </label>
    <div class="form-group">
      <span class="form-group-label">Purpose</span>
      <div class="chips">
        {#each purposes as p}
          {#if !modalPath.trim()}
            <span class="chip disabled">{p.label}</span>
          {:else if p.label !== 'Custom' && profiles.some(pr => pr.projectPath === modalPath && pr.purpose === p.label)}
            <span class="chip disabled" title="{p.label} already active for this project">{p.label}</span>
          {:else}
            <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
            <span class="chip" class:selected={modalPurpose === p.label}
              style={modalPurpose === p.label ? `background:${p.color}33;color:${p.color};border-color:${p.color}` : ''}
              onclick={() => { modalPurpose = p.label; if (p.label === 'Custom' && modalPath.trim()) loadExistingSessions(modalPath); }}>{p.label}</span>
          {/if}
        {/each}
      </div>
    </div>
    {#if modalExistingSessions.length > 0 && modalPurpose !== 'Custom'}
      <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
      <div class="session-found-hint" onclick={() => { modalPurpose = 'Custom'; }}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
        <span>{modalExistingSessions.length} previous session{modalExistingSessions.length > 1 ? 's' : ''} found — <strong style="color:var(--accent);cursor:pointer;">resume via Custom</strong></span>
      </div>
    {/if}
    {#if modalPurpose === 'Custom'}
      {#if modalExistingSessions.length > 0}
        <label>Resume Existing Session
          <select class="session-select" bind:value={modalSelectedSession}>
            <option value="">Start fresh</option>
            {#each modalExistingSessions as s}
              <option value={s.sessionId}>{s.preview || s.sessionId.slice(0, 8)} — {new Date(s.modifiedAt).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</option>
            {/each}
          </select>
        </label>
      {/if}
      <label>System Prompt <span style="font-size:10px;color:var(--text-secondary);font-weight:normal;">(optional)</span>
        <textarea class="custom-prompt" bind:value={modalCustomPrompt} placeholder="Custom instructions for this session..." rows="2"></textarea>
      </label>
    {/if}

    <div class="stg-section-label" style="margin-top:12px;margin-bottom:6px;">Advanced</div>
    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
    <div class="toggle-row">
      <span class="toggle-label">Skip permissions
        <span class="toggle-tooltip">?</span>
      </span>
      <button class="toggle-switch" class:on={modalSkipPermissions} onclick={() => modalSkipPermissions = !modalSkipPermissions}>
        <span class="toggle-knob"></span>
      </button>
    </div>
    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
    <div class="toggle-row">
      <span class="toggle-label">Git Identity</span>
      <button class="toggle-switch" class:on={modalGitEnabled} onclick={() => modalGitEnabled = !modalGitEnabled}>
        <span class="toggle-knob"></span>
      </button>
    </div>
    {#if modalGitEnabled}
      <div class="advanced-section">
        <div class="advanced-row">
          <label class="advanced-label">Name <span class="required">*</span>
            <input type="text" bind:value={modalGitName} placeholder="e.g. John Doe" class="advanced-input" />
          </label>
          <label class="advanced-label">Email <span class="required">*</span>
            <input type="email" bind:value={modalGitEmail} placeholder="e.g. john@example.com" class="advanced-input" />
          </label>
        </div>
      </div>
    {/if}
    <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
    <div class="toggle-row">
      <span class="toggle-label">Attach Contexts</span>
      <button class="toggle-switch" class:on={contextsStore.modalContextEnabled} onclick={() => contextsStore.modalContextEnabled = !contextsStore.modalContextEnabled}>
        <span class="toggle-knob"></span>
      </button>
    </div>
    {#if contextsStore.modalContextEnabled}
      <div class="advanced-section">
        {#if contextsStore.modalContexts.length > 0}
          <div class="ctx-attached-chips">
            {#each contextsStore.modalContexts as name}
              <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
              <span class="ctx-attached-chip">
                {name}
                <span class="ctx-chip-remove" onclick={() => { contextsStore.modalContexts = contextsStore.modalContexts.filter(c => c !== name); }}>×</span>
              </span>
            {/each}
          </div>
        {/if}
        <div class="ctx-add-wrap">
          <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
          <button class="ctx-add-btn" onclick={(e) => { e.stopPropagation(); contextsStore.showContextDropdown = !contextsStore.showContextDropdown; }}>
            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor"><path d="M7.75 2a.75.75 0 01.75.75V7h4.25a.75.75 0 010 1.5H8.5v4.25a.75.75 0 01-1.5 0V8.5H2.75a.75.75 0 010-1.5H7V2.75A.75.75 0 017.75 2z"/></svg>
            Add
          </button>
          {#if contextsStore.showContextDropdown}
            <div class="ctx-dropdown">
              {#each contextsStore.contextSnippets.filter(c => !contextsStore.modalContexts.includes(c.name)) as ctx}
                <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
                <div class="ctx-dropdown-item" onclick={() => { contextsStore.modalContexts = [...contextsStore.modalContexts, ctx.name]; contextsStore.showContextDropdown = false; }}>
                  <span class="ctx-dropdown-name">{ctx.name}</span>
                  <span class="ctx-dropdown-preview">{ctx.preview}</span>
                </div>
              {:else}
                <div class="ctx-dropdown-empty">No more contexts available</div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}
    <div class="modal-actions">
      <button onclick={handleCancel}>Cancel</button>
      <button class="create-btn" disabled={!modalPath || !modalTitle || !modalPurpose || (modalGitEnabled && (!modalGitName.trim() || !modalGitEmail.trim()))} onclick={handleCreate}>Create</button>
    </div>
  </div>
</div>
{/if}

<style>
  .modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 1000; animation: fadeIn 0.15s ease-out; }
  .modal { background: #161b22; border: 1px solid var(--border); border-radius: 12px; padding: 20px; width: 420px; max-width: 90vw; animation: slideIn 0.2s ease-out; }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideIn { from { opacity: 0; transform: translateY(-10px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
  .modal h2 { font-size: 15px; color: var(--text-primary); margin: 0 0 16px; }
  .modal label { display: block; font-size: 12px; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 12px; }
  .modal input { width: 100%; background: var(--input-bg, #0d1117); border: 1px solid var(--border); border-radius: 6px; padding: 8px 10px; font-size: 13px; color: var(--text-primary); outline: none; box-sizing: border-box; margin-top: 4px; }
  .modal input:focus { border-color: var(--accent); }
  .row { display: flex; gap: 8px; margin-top: 4px; }
  .row input { flex: 1; margin-top: 0; }
  .row button { background: var(--btn-bg, #21262d); border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; color: var(--text-primary); font-size: 13px; cursor: pointer; white-space: nowrap; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 6px; }
  .form-group { display: flex; flex-direction: column; gap: 6px; }
  .form-group-label { font-size: 13px; color: var(--text-primary); }
  .chip { padding: 5px 12px; border-radius: 14px; border: 1px solid var(--border); background: transparent; color: var(--text-secondary); font-size: 12px; cursor: pointer; font-family: inherit; transition: background 0.15s, color 0.15s; user-select: none; display: inline-block; }
  .chip:hover:not(.selected):not(.disabled) { background: rgba(255,255,255,0.06); }
  .chip.disabled { opacity: 0.3; cursor: not-allowed; }
  .chip.selected { font-weight: 600; border-color: var(--accent); color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, transparent); }
  .session-select { width: 100%; margin-top: 6px; padding: 7px 10px; border-radius: 6px; border: 1px solid var(--border); background: var(--btn-bg, #21262d); color: var(--text-primary); font-size: 12px; font-family: inherit; appearance: none; cursor: pointer; }
  .session-select option { background: #1c2128; color: var(--text-primary); }
  .custom-prompt { width: 100%; margin-top: 6px; padding: 8px 10px; border-radius: 6px; border: 1px solid var(--border); background: var(--btn-bg, #21262d); color: var(--text-primary); font-size: 12px; font-family: inherit; resize: vertical; min-height: 60px; line-height: 1.5; }
  .custom-prompt::placeholder { color: var(--text-secondary); }
  .advanced-section { padding: 4px 0 0; display: flex; flex-direction: column; gap: 8px; animation: advIn 0.12s ease; }
  @keyframes advIn { from { opacity: 0; } to { opacity: 1; } }
  .advanced-row { display: flex; gap: 8px; }
  .advanced-row > label { flex: 1; }
  .advanced-label { font-size: 11px; color: var(--text-secondary); display: flex; flex-direction: column; gap: 4px; }
  .advanced-input { padding: 6px 8px; border-radius: 6px; border: 1px solid var(--border); background: transparent; color: var(--text-primary); font-size: 12px; font-family: inherit; }
  .advanced-input:focus { border-color: var(--accent); outline: none; }
  .advanced-input::placeholder { color: var(--text-secondary); opacity: 0.6; }
  .required { color: #f85149; font-weight: 600; }
  .session-found-hint { display: flex; align-items: flex-start; gap: 8px; padding: 8px 10px; border-radius: 6px; background: color-mix(in srgb, var(--accent) 8%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 20%, transparent); margin: 8px 0; }
  .session-found-hint svg { flex-shrink: 0; margin-top: 1px; }
  .session-found-hint span { font-size: 11px; color: var(--text-secondary); line-height: 1.4; }
  .toggle-row { display: flex; align-items: center; justify-content: space-between; margin-top: 12px; }
  .toggle-label { font-size: 12px; color: var(--text-secondary); display: flex; align-items: center; gap: 6px; }
  .toggle-tooltip { display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px; border-radius: 50%; border: 1px solid var(--border); font-size: 10px; color: var(--text-secondary); cursor: default; position: relative; }
  .toggle-tooltip:hover::after { content: 'Bypasses all permission prompts. Claude will execute commands, edit files, and make changes without asking.'; position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%); background: #1c2128; border: 1px solid var(--border); border-radius: 6px; padding: 6px 10px; font-size: 11px; color: var(--text-primary); white-space: normal; width: 220px; box-shadow: 0 4px 12px rgba(0,0,0,0.3); z-index: 100; line-height: 1.4; }
  .toggle-switch { width: 36px; height: 20px; border-radius: 10px; border: 1px solid var(--border); background: rgba(255,255,255,0.06); cursor: pointer; position: relative; transition: all 0.2s; padding: 0; }
  .toggle-switch.on { background: var(--accent); border-color: var(--accent); }
  .toggle-knob { position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: var(--text-secondary); transition: all 0.2s; }
  .toggle-switch.on .toggle-knob { left: 18px; background: #fff; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  .modal-actions button { padding: 7px 16px; border-radius: 6px; font-size: 13px; cursor: pointer; border: 1px solid var(--border); background: var(--btn-bg, #21262d); color: var(--text-primary); font-family: inherit; }
  .create-btn { background: var(--accent) !important; border-color: transparent !important; color: #fff !important; }
  .create-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .stg-section-label { font-size: 11px; font-weight: 600; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.1em; }
  .ctx-attached-chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .ctx-attached-chip { display: flex; align-items: center; gap: 4px; padding: 3px 6px 3px 10px; border-radius: 12px; background: color-mix(in srgb, var(--accent) 12%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent); color: var(--accent); font-size: 11px; font-weight: 500; }
  .ctx-chip-remove { cursor: pointer; font-size: 14px; line-height: 1; opacity: 0.6; transition: opacity 0.1s; display: flex; align-items: center; }
  .ctx-chip-remove:hover { opacity: 1; }
  .ctx-add-wrap { position: relative; }
  .ctx-add-btn { display: flex; align-items: center; gap: 4px; padding: 4px 10px; border-radius: 4px; border: 1px dashed var(--border); background: transparent; color: var(--text-secondary); font-size: 11px; font-family: inherit; cursor: pointer; transition: all 0.1s; }
  .ctx-add-btn:hover { border-color: var(--accent); color: var(--accent); }
  .ctx-dropdown { position: absolute; top: calc(100% + 4px); left: 0; width: 250px; background: var(--sidebar-bg); border: 1px solid var(--border); border-radius: 6px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); z-index: 100; max-height: 180px; overflow-y: auto; padding: 4px; }
  .ctx-dropdown-item { padding: 6px 10px; border-radius: 4px; cursor: pointer; transition: background 0.1s; }
  .ctx-dropdown-item:hover { background: rgba(255,255,255,0.06); }
  .ctx-dropdown-name { font-size: 12px; font-weight: 500; color: var(--text-primary); display: block; }
  .ctx-dropdown-preview { font-size: 10px; color: var(--text-secondary); display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ctx-dropdown-empty { padding: 10px; text-align: center; font-size: 11px; color: var(--text-secondary); }
</style>
