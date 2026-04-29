<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { SearchAddon } from '@xterm/addon-search';
  import '@xterm/xterm/css/xterm.css';
  import { Channel } from '@tauri-apps/api/core';
  import {
    activeSshProfile,
    sshProfiles,
    sshTerminalIds,
    sshConnStates,
    loadSshProfiles,
  } from '../stores';
  import {
    sshSpawnTerminal,
    sshWriteToTerminal,
    sshResizeTerminal,
    sshKillTerminal,
    sshTouchProfile,
  } from '../commands';
  import { tabs as tabsStore, addTab, activateTab, closeTab } from '$lib/shared/stores/tabs';
  import { getTerminalTheme } from '$lib/utils/theme';
  import { appearance } from '$lib/stores/settings';
  import { showToast } from '$lib/shared/primitives/toast';
  import { base64ToBytes, deferUntilFrame, loadWebGLAddon } from '$lib/shared/primitives/terminal-utils';
  import { resolveSshCapture, rejectAllSshCaptures, type SshCaptureRequest } from '../ai/execute';
  import type { SshProfile, SshTerminalPayload } from '../types';
  import { SSH_EVENT } from '$lib/shared/constants/events';
  import { RESIZE_DEBOUNCE_MS, SSH_CAPTURE_TIMEOUT_MS } from '$lib/shared/constants/timings';
  import {
    SEARCH_MATCH_BG,
    SEARCH_MATCH_BORDER,
    SEARCH_ACTIVE_MATCH_BG,
    SEARCH_ACTIVE_MATCH_BORDER,
  } from '$lib/shared/constants/colors';

  let terminalEl: HTMLDivElement;

  // Per-tab xterm entry. Keyed by tab.key (== profile.id-based key).
  type TermEntry = {
    term: Terminal;
    fitAddon: FitAddon;
    searchAddon: SearchAddon;
    container: HTMLDivElement;
    terminalId: string | null;
    profileId: string;
    tabKey: string;
    generation: number;
    // Active capture for execute_shell tool. Null when no AI command pending.
    capture: {
      requestId: string;
      buffer: string;
      timeoutId: ReturnType<typeof setTimeout>;
    } | null;
  };

  // Search-decoration options passed to SearchAddon. The addon does not honor
  // CSS custom properties, so we use the resolved tokens from constants/colors.
  const SEARCH_DECORATIONS = {
    matchBackground: SEARCH_MATCH_BG,
    matchBorder: SEARCH_MATCH_BORDER,
    matchOverviewRuler: SEARCH_MATCH_BORDER,
    activeMatchBackground: SEARCH_ACTIVE_MATCH_BG,
    activeMatchBorder: SEARCH_ACTIVE_MATCH_BORDER,
    activeMatchColorOverviewRuler: SEARCH_ACTIVE_MATCH_BORDER,
  } as const;

  // Heuristic shell prompt detector: matches `$ `, `# `, `> `, `% `, `❯ ` at the
  // end of the cleaned (ANSI-stripped) buffer. Imperfect — some PS1 setups omit
  // the trailing space — but the 15s timeout is the hard backstop.
  const SHELL_PROMPT_RE = /[\$#>❯%]\s*$/m;
  const ANSI_RE = /\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*(?:\x07|\x1b\\)/g;
  const CAPTURE_TIMEOUT_MS = SSH_CAPTURE_TIMEOUT_MS;
  const CAPTURE_MAX_CHARS = 100_000;

  function stripAnsi(text: string): string {
    return text.replace(ANSI_RE, '');
  }

  function finishCapture(entry: TermEntry, reason: 'prompt' | 'timeout' | 'cleanup' = 'prompt') {
    const cap = entry.capture;
    if (!cap) return;
    clearTimeout(cap.timeoutId);
    entry.capture = null;
    const cleaned = stripAnsi(cap.buffer).trim();
    const note = reason === 'timeout' ? '\n[NOTE] Capture timed out after 15s; output may be incomplete.' : '';
    resolveSshCapture(cap.requestId, cleaned + note);
  }

  // Map keyed by tabKey (we use profile.id+timestamp for uniqueness).
  const termEntries = new Map<string, TermEntry>();
  let activeEntry: TermEntry | null = null;

  // Track which tabs have an exited terminal (for reconnect banner).
  let exitedTabs = $state<Set<string>>(new Set());

  // Loading state — gated by first-data-received flag for current spawn.
  let spawning = $state(false);
  let termReady = $state(false);

  // Terminal background color (synced with theme).
  let termBg = $state('#0d0d18');

  // Last-active tab.key tracked here so the activeAgent-style subscriber
  // knows whether re-entry is a real switch or noop.
  let currentTabKey: string | null = null;

  // Per-tab generation: invalidates stale Channel writes after reconnect.
  const generations = new Map<string, number>();

  function getCurrentTermTheme(): Record<string, string> {
    const app = get(appearance);
    return getTerminalTheme(app.theme, app.accentColor);
  }

  function createEntry(tabKey: string, profile: SshProfile): TermEntry {
    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", "SF Mono", "Menlo", monospace',
      theme: getCurrentTermTheme(),
      scrollback: 10000,
      lineHeight: 1.35,
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    const searchAddon = new SearchAddon();
    term.loadAddon(searchAddon);
    // Track match position for the "x of N" counter shown in the search bar.
    searchAddon.onDidChangeResults(({ resultIndex, resultCount }) => {
      searchMatchIndex = resultIndex;
      searchMatchTotal = resultCount;
    });

    const container = document.createElement('div');
    container.style.cssText = 'width:100%;height:100%;display:none;';
    terminalEl.appendChild(container);
    term.open(container);
    loadWebGLAddon(term);

    const entry: TermEntry = {
      term,
      fitAddon,
      searchAddon,
      container,
      terminalId: null,
      profileId: profile.id,
      tabKey,
      generation: 0,
      capture: null,
    };

    term.onData((data) => {
      const id = entry.terminalId;
      if (!id) return;
      sshWriteToTerminal(id, data).catch(() => {
        // PTY/channel dead — mark exited and surface reconnect banner
        markExited(tabKey);
      });
    });

    let resizeTimer: ReturnType<typeof setTimeout> | null = null;
    new ResizeObserver(() => {
      if (!fitAddon || container.offsetWidth <= 0) return;
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        resizeTimer = null;
        try {
          fitAddon.fit();
          if (entry.terminalId) {
            const dims = fitAddon.proposeDimensions();
            if (dims) sshResizeTerminal(entry.terminalId, dims.cols, dims.rows).catch(() => {});
          }
        } catch {
          /* ignore */
        }
      }, RESIZE_DEBOUNCE_MS);
    }).observe(container);

    termEntries.set(tabKey, entry);
    return entry;
  }

  function showEntry(entry: TermEntry) {
    if (activeEntry && activeEntry !== entry) {
      activeEntry.container.style.display = 'none';
      try { activeEntry.term.options.scrollback = 1000; } catch { /* ignore */ }
    }
    entry.container.style.display = 'block';
    try { entry.term.options.scrollback = 10000; } catch { /* ignore */ }
    activeEntry = entry;
    requestAnimationFrame(() => {
      try { entry.fitAddon.fit(); } catch { /* ignore */ }
      try { entry.term.focus(); } catch { /* ignore */ }
    });
  }

  function markExited(tabKey: string) {
    const entry = termEntries.get(tabKey);
    if (entry) entry.terminalId = null;
    sshTerminalIds.update((m) => {
      m.delete(tabKey);
      return new Map(m);
    });
    sshConnStates.update((m) => {
      m.set(tabKey, 'disconnected');
      return new Map(m);
    });
    exitedTabs = new Set([...exitedTabs, tabKey]);
  }

  async function spawnFor(entry: TermEntry, profile: SshProfile) {
    // Bump generation so older stale Channel callbacks no-op.
    const gen = (generations.get(entry.tabKey) ?? 0) + 1;
    generations.set(entry.tabKey, gen);
    entry.generation = gen;

    spawning = true;
    termReady = false;
    sshConnStates.update((m) => {
      m.set(entry.tabKey, 'connecting');
      return new Map(m);
    });
    exitedTabs = new Set([...exitedTabs].filter((k) => k !== entry.tabKey));

    let firstDataSeen = false;
    const channel = new Channel<SshTerminalPayload>();
    channel.onmessage = (payload) => {
      if (entry.generation !== gen) return; // stale spawn
      if (payload.exit === true) {
        // If exit arrives DURING spawn (e.g. auth ok but shell died instantly,
        // or the server closed the channel right after request_shell), the
        // loader would otherwise stay forever — flip flags so only the banner
        // is shown, not loader-and-banner together.
        if (spawning) {
          spawning = false;
          termReady = false;
        }
        markExited(entry.tabKey);
        try {
          entry.term.write('\r\n\x1b[33m[connection closed]\x1b[0m\r\n');
        } catch { /* ignore */ }
        return;
      }
      if (!firstDataSeen && payload.data) {
        firstDataSeen = true;
        deferUntilFrame(() => {
          if (spawning) {
            spawning = false;
            termReady = true;
          }
        });
      }
      if (payload.data) {
        try {
          const bytes = base64ToBytes(payload.data);
          entry.term.write(bytes);
          // Capture buffer for execute_shell tool: append decoded text and
          // check for shell prompt at end (heuristic stop).
          if (entry.capture) {
            const text = new TextDecoder().decode(bytes);
            entry.capture.buffer += text;
            if (entry.capture.buffer.length > CAPTURE_MAX_CHARS) {
              entry.capture.buffer = entry.capture.buffer.slice(-CAPTURE_MAX_CHARS);
            }
            const cleaned = stripAnsi(entry.capture.buffer);
            if (SHELL_PROMPT_RE.test(cleaned)) {
              finishCapture(entry, 'prompt');
            }
          }
        } catch { /* ignore decode errors */ }
      }
    };

    try {
      const terminalId = await sshSpawnTerminal(profile.id, channel);
      entry.terminalId = terminalId;
      sshTerminalIds.update((m) => {
        m.set(entry.tabKey, terminalId);
        return new Map(m);
      });
      sshConnStates.update((m) => {
        m.set(entry.tabKey, 'connected');
        return new Map(m);
      });
      // Rust just bumped last_used_at as part of spawn — refresh the store so
      // the SshNav list reflects the new "Xs ago" instead of "never".
      loadSshProfiles().catch(() => {});

      // Send initial fit
      requestAnimationFrame(() => {
        try {
          entry.fitAddon.fit();
          const dims = entry.fitAddon.proposeDimensions();
          if (dims) sshResizeTerminal(terminalId, dims.cols, dims.rows).catch(() => {});
        } catch { /* ignore */ }
      });
    } catch (e) {
      spawning = false;
      termReady = false;
      const msg = String(e);
      // Distinguish timeout from other failures so the user gets clear feedback.
      const isTimeout = msg.toLowerCase().includes('timed out') || msg.toLowerCase().includes('timeout');
      // Clean up the local entry — connection never came up, no PTY to keep around.
      try { entry.container.remove(); } catch { /* ignore */ }
      try { entry.term.dispose(); } catch { /* ignore */ }
      termEntries.delete(entry.tabKey);
      sshConnStates.update((m) => { m.set(entry.tabKey, 'disconnected'); return new Map(m); });
      // Close the tab and return to home so the user can pick a profile and retry.
      const allTabs = get(tabsStore);
      const tab = allTabs.find((t) => t.mode === 'ssh' && t.key === entry.tabKey);
      if (tab) closeTab(tab.id);
      activeSshProfile.set(null);
      showToast(isTimeout ? `Connection to ${profile.host} timed out` : `Failed to connect: ${msg}`, 'error');
    }
  }

  async function activateProfile(profile: SshProfile) {
    if (!terminalEl) return;
    const tabKey = profile.id;

    // Re-attach existing entry if still alive
    let entry = termEntries.get(tabKey);
    if (entry && entry.terminalId) {
      if (entry.container.parentElement !== terminalEl) {
        terminalEl.appendChild(entry.container);
      }
      currentTabKey = tabKey;
      spawning = false;
      termReady = true;
      showEntry(entry);
      return;
    }

    if (entry) {
      // Stale — recreate xterm to avoid showing prior buffer for a new connection
      try { entry.container.remove(); } catch { /* ignore */ }
      try { entry.term.dispose(); } catch { /* ignore */ }
      termEntries.delete(tabKey);
    }

    currentTabKey = tabKey;
    entry = createEntry(tabKey, profile);
    showEntry(entry);
    await spawnFor(entry, profile);
  }

  // Cancel an in-flight connect attempt. We don't have a Rust-side abort
  // handle (russh's connect future isn't easily abortable from outside), so
  // the strategy is: kill the terminal id if it was issued, otherwise just
  // tear down the local tab so the user is unblocked. Backend timeout (15s)
  // is the hard floor.
  async function cancelConnect() {
    const profile = get(activeSshProfile);
    if (!profile) return;
    const tabKey = profile.id;
    const entry = termEntries.get(tabKey);
    if (entry?.terminalId) {
      sshKillTerminal(entry.terminalId).catch(() => {});
    }
    if (entry) {
      try { entry.container.remove(); } catch { /* ignore */ }
      try { entry.term.dispose(); } catch { /* ignore */ }
      termEntries.delete(tabKey);
    }
    spawning = false;
    termReady = false;
    sshConnStates.update((m) => { m.set(tabKey, 'disconnected'); return new Map(m); });
    // Close the tab and unset active profile — return user to home screen.
    const allTabs = get(tabsStore);
    const tab = allTabs.find((t) => t.mode === 'ssh' && t.key === tabKey);
    if (tab) closeTab(tab.id);
    activeSshProfile.set(null);
    showToast('Connection cancelled', 'info');
  }

  async function reconnectActive() {
    const profile = get(activeSshProfile);
    if (!profile) return;
    const tabKey = profile.id;
    const entry = termEntries.get(tabKey);
    if (!entry) {
      activateProfile(profile);
      return;
    }
    // Kill any lingering remote terminal first
    if (entry.terminalId) {
      sshKillTerminal(entry.terminalId).catch(() => {});
      entry.terminalId = null;
    }
    try { entry.term.clear(); } catch { /* ignore */ }
    await spawnFor(entry, profile);
  }

  // ── Event listeners ─────────────────────────────────────────────────────────

  function handleOpenTab(e: Event) {
    const profile = (e as CustomEvent<SshProfile>).detail;
    if (!profile) return;

    const all = get(tabsStore);
    const existing = all.find((t) => t.mode === 'ssh' && t.key === profile.id);
    if (existing) {
      activateTab(existing.id);
    } else {
      addTab(profile.name, 'ssh', profile.id, 'var(--ssh)');
    }
    activeSshProfile.set(profile);
    // Bump last_used_at on every open path (NewProfileModal save, Topbar +
    // picker, SshNav click) so "last used" reflects reality everywhere.
    sshTouchProfile(profile.id)
      .then(() => loadSshProfiles())
      .catch(() => {});
  }

  function handleCloseTab(e: Event) {
    const detail = (e as CustomEvent).detail;
    const tabKey = detail?.tabKey as string | undefined;
    if (!tabKey) return;

    // Bump generation so any in-flight Channel callbacks for this tabKey
    // (mid-connection writes/exit) no-op against a removed entry.
    generations.set(tabKey, (generations.get(tabKey) ?? 0) + 1);

    const wasActive = activeEntry?.tabKey === tabKey
      || get(activeSshProfile)?.id === tabKey;

    const entry = termEntries.get(tabKey);
    if (entry) {
      if (entry.terminalId) sshKillTerminal(entry.terminalId).catch(() => {});
      try { entry.container.remove(); } catch { /* ignore */ }
      try { entry.term.dispose(); } catch { /* ignore */ }
      termEntries.delete(tabKey);
    }
    sshTerminalIds.update((m) => {
      m.delete(tabKey);
      return new Map(m);
    });
    sshConnStates.update((m) => {
      m.delete(tabKey);
      return new Map(m);
    });
    exitedTabs = new Set([...exitedTabs].filter((k) => k !== tabKey));

    if (wasActive) {
      activeEntry = null;
      currentTabKey = null;
      // Reset loader/banner flags so whatever Topbar activates next (or the
      // empty home screen) renders cleanly. Topbar owns the active-profile
      // switch — we don't double-set it here to avoid racing with Topbar's
      // own next-tab selection.
      spawning = false;
      termReady = false;
    }
  }

  function handleInsertCommand(e: Event) {
    const cmd = (e as CustomEvent<string>).detail;
    if (!cmd || typeof cmd !== 'string') return;
    if (!activeEntry?.terminalId) {
      showToast('No active SSH terminal', 'info');
      return;
    }
    // Insert command at cursor without trailing newline. User presses Enter
    // themselves. Used as a fallback when AI sends a code block instead of
    // calling execute_shell (e.g. for interactive commands the system prompt
    // says shouldn't be auto-run).
    sshWriteToTerminal(activeEntry.terminalId, cmd).catch(() => {
      showToast('Failed to write to terminal', 'error');
    });
    try {
      activeEntry.term.focus();
    } catch { /* ignore */ }
  }

  // ── Reactive subscriptions ──────────────────────────────────────────────────

  const unsubProfile = activeSshProfile.subscribe((profile) => {
    if (profile && profile.id !== currentTabKey) {
      requestAnimationFrame(() => activateProfile(profile));
    } else if (!profile) {
      currentTabKey = null;
      if (activeEntry) {
        activeEntry.container.style.display = 'none';
        activeEntry = null;
      }
    }
  });

  const unsubAppearance = appearance.subscribe((app) => {
    if (!app) return;
    const theme = getTerminalTheme(app.theme, app.accentColor);
    termBg = theme.background || '#0d0d18';
    for (const entry of termEntries.values()) {
      try { entry.term.options.theme = theme; } catch { /* ignore */ }
    }
  });

  function handleExecuteCaptureRequest(e: Event) {
    const detail = (e as CustomEvent<SshCaptureRequest>).detail;
    if (!detail) return;
    const { requestId, profileId, command } = detail;
    // Find the entry whose profile matches. Prefer the active entry.
    let target: TermEntry | null = null;
    if (activeEntry && activeEntry.profileId === profileId && activeEntry.terminalId) {
      target = activeEntry;
    } else {
      for (const entry of termEntries.values()) {
        if (entry.profileId === profileId && entry.terminalId) { target = entry; break; }
      }
    }
    if (!target || !target.terminalId) {
      resolveSshCapture(requestId, '[ERROR] No live SSH terminal for the requested profile.');
      return;
    }
    // Already capturing for another tool call — reject the older one.
    if (target.capture) {
      finishCapture(target, 'cleanup');
    }
    target.capture = {
      requestId,
      buffer: '',
      timeoutId: setTimeout(() => {
        const e2 = target!;
        if (e2.capture && e2.capture.requestId === requestId) {
          finishCapture(e2, 'timeout');
        }
      }, CAPTURE_TIMEOUT_MS),
    };
    // Write the command followed by Enter.
    sshWriteToTerminal(target.terminalId, command + '\r').catch(() => {
      finishCapture(target!, 'cleanup');
      resolveSshCapture(requestId, '[ERROR] Failed to write command to SSH terminal.');
    });
  }

  onMount(async () => {
    window.addEventListener(SSH_EVENT.OPEN_TAB, handleOpenTab);
    window.addEventListener(SSH_EVENT.CLOSE_TAB, handleCloseTab);
    window.addEventListener(SSH_EVENT.INSERT_COMMAND, handleInsertCommand);
    window.addEventListener(SSH_EVENT.EXECUTE_CAPTURE_REQUEST, handleExecuteCaptureRequest);

    // First mount: load profiles + auto-attach if there's a tab waiting.
    await loadSshProfiles();

    // If a tab is already active for the current mode, restore the session.
    const profile = get(activeSshProfile);
    if (profile) {
      currentTabKey = null; // force activate
      requestAnimationFrame(() => activateProfile(profile));
    } else {
      // If there is an SSH tab but no active profile, hydrate it.
      const all = get(tabsStore);
      const sshTab = all.find((t) => t.mode === 'ssh' && t.key);
      if (sshTab?.key) {
        const profiles = get(sshProfiles);
        const match = profiles.find((p) => p.id === sshTab.key);
        if (match) activeSshProfile.set(match);
      }
    }
  });

  onDestroy(() => {
    unsubProfile();
    unsubAppearance();
    window.removeEventListener(SSH_EVENT.OPEN_TAB, handleOpenTab);
    window.removeEventListener(SSH_EVENT.CLOSE_TAB, handleCloseTab);
    window.removeEventListener(SSH_EVENT.INSERT_COMMAND, handleInsertCommand);
    window.removeEventListener(SSH_EVENT.EXECUTE_CAPTURE_REQUEST, handleExecuteCaptureRequest);
    rejectAllSshCaptures('SSH panel unmounted');
  });

  // Reconnect banner state for active tab
  let activeIsExited = $derived(
    !!$activeSshProfile && exitedTabs.has($activeSshProfile.id)
  );

  // ── Find-in-terminal (Cmd+F / Ctrl+F) ─────────────────────────────────────
  let searchOpen = $state(false);
  let searchQuery = $state('');
  let searchMatchIndex = $state(-1);
  let searchMatchTotal = $state(0);
  let searchInputEl: HTMLInputElement | null = $state(null);

  // Mac uses Cmd, other platforms use Ctrl. Mirrors how the rest of the app
  // distinguishes via metaKey vs ctrlKey (see src/lib/utils/shortcuts.ts).
  const isMac = typeof navigator !== 'undefined' && navigator.platform.startsWith('Mac');

  function openSearch() {
    if (!activeEntry) return;
    searchOpen = true;
    requestAnimationFrame(() => {
      searchInputEl?.focus();
      searchInputEl?.select();
    });
  }

  function closeSearch() {
    searchOpen = false;
    searchQuery = '';
    searchMatchIndex = -1;
    searchMatchTotal = 0;
    activeEntry?.searchAddon.clearDecorations();
    try { activeEntry?.term.focus(); } catch { /* ignore */ }
  }

  function findNext() {
    if (!activeEntry || !searchQuery) return;
    activeEntry.searchAddon.findNext(searchQuery, { decorations: SEARCH_DECORATIONS });
  }

  function findPrevious() {
    if (!activeEntry || !searchQuery) return;
    activeEntry.searchAddon.findPrevious(searchQuery, { decorations: SEARCH_DECORATIONS });
  }

  function handleSearchInput(e: Event) {
    searchQuery = (e.target as HTMLInputElement).value;
    if (!activeEntry) return;
    if (searchQuery.length === 0) {
      activeEntry.searchAddon.clearDecorations();
      searchMatchIndex = -1;
      searchMatchTotal = 0;
      return;
    }
    activeEntry.searchAddon.findNext(searchQuery, { decorations: SEARCH_DECORATIONS });
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeSearch();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) findPrevious();
      else findNext();
    }
  }

  function handleSshKeydown(e: KeyboardEvent) {
    // Only fire when SSH mode is active and there's a live terminal entry.
    // Scoping by activeEntry naturally limits this to "SSH panel has focus";
    // global Cmd+F outside SSH never reaches this listener (it lives on the
    // ssh-panel element).
    const meta = isMac ? e.metaKey : e.ctrlKey;
    if (meta && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
      if (!activeEntry) return;
      e.preventDefault();
      e.stopPropagation();
      if (searchOpen) {
        searchInputEl?.focus();
        searchInputEl?.select();
      } else {
        openSearch();
      }
    }
  }

  // When the active tab changes (subscriber on activeSshProfile already fires),
  // close any open search bar — it's tied to the previous tab's terminal.
  $effect(() => {
    void $activeSshProfile;
    if (searchOpen) closeSearch();
  });
</script>

{#if $activeSshProfile}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ssh-panel" onkeydown={handleSshKeydown} tabindex="-1">
    {#if searchOpen}
      <div class="ssh-search">
        <svg class="ssh-search-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="11" cy="11" r="8"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          bind:this={searchInputEl}
          class="ssh-search-input"
          type="text"
          placeholder="Find in terminal…"
          value={searchQuery}
          oninput={handleSearchInput}
          onkeydown={handleSearchKeydown}
        />
        <span class="ssh-search-count">
          {#if searchQuery && searchMatchTotal > 0}
            {searchMatchIndex + 1} of {searchMatchTotal}
          {:else if searchQuery}
            No results
          {/if}
        </span>
        <button class="ssh-search-btn" title="Previous match (Shift+Enter)" onclick={findPrevious} disabled={!searchQuery || searchMatchTotal === 0}>
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="18 15 12 9 6 15"/>
          </svg>
        </button>
        <button class="ssh-search-btn" title="Next match (Enter)" onclick={findNext} disabled={!searchQuery || searchMatchTotal === 0}>
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9"/>
          </svg>
        </button>
        <button class="ssh-search-btn ssh-search-close" title="Close (Esc)" onclick={closeSearch}>
          <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    {/if}
    {#if spawning}
      <div class="ssh-loading">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--ssh)" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="4" width="20" height="6" rx="1"/>
          <rect x="2" y="14" width="20" height="6" rx="1"/>
          <line x1="6" y1="7" x2="6.01" y2="7"/>
          <line x1="6" y1="17" x2="6.01" y2="17"/>
        </svg>
        <div class="loading-text">
          <span class="loading-title">Connecting to {$activeSshProfile.host}</span>
          <span class="loading-sub">{$activeSshProfile.username}@{$activeSshProfile.host}:{$activeSshProfile.port}<span class="loading-dots"></span></span>
        </div>
        <button class="ssh-cancel-btn" onclick={cancelConnect}>Cancel</button>
      </div>
    {/if}

    {#if activeIsExited}
      <div class="ssh-banner">
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="var(--err)" stroke-width="2" stroke-linecap="round">
          <circle cx="12" cy="12" r="10"/>
          <line x1="15" y1="9" x2="9" y2="15"/>
          <line x1="9" y1="9" x2="15" y2="15"/>
        </svg>
        <span class="ssh-banner-text">Connection closed</span>
        <button class="ssh-banner-btn" onclick={reconnectActive}>Reconnect</button>
      </div>
    {/if}

    <div class="ssh-terminal-container" class:term-hidden={!termReady && !activeIsExited} bind:this={terminalEl} style="background:{termBg}"></div>
  </div>
{:else}
  <div class="ssh-empty">
    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--t4)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2" y="4" width="20" height="6" rx="1"/>
      <rect x="2" y="14" width="20" height="6" rx="1"/>
      <line x1="6" y1="7" x2="6.01" y2="7"/>
      <line x1="6" y1="17" x2="6.01" y2="17"/>
    </svg>
    <p class="empty-title">No active SSH session</p>
    <p class="empty-sub">Pick a profile from the sidebar or create a new one</p>
  </div>
{/if}

<style>
  .ssh-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }
  .ssh-terminal-container {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    transition: opacity 0.15s ease;
  }
  .ssh-terminal-container.term-hidden { opacity: 0; }
  .ssh-terminal-container :global(.xterm) { height: 100% !important; padding: 0 !important; }
  .ssh-terminal-container :global(.xterm-viewport) { height: 100% !important; scrollbar-gutter: auto; }
  .ssh-terminal-container :global(.xterm-screen) { height: 100% !important; }
  .ssh-terminal-container :global(.xterm-viewport::-webkit-scrollbar) { width: 3px; }
  .ssh-terminal-container :global(.xterm-viewport::-webkit-scrollbar-track) { background: transparent; }
  .ssh-terminal-container :global(.xterm-viewport::-webkit-scrollbar-thumb) { background: rgba(255,255,255,0.10); border-radius: 3px; }
  .ssh-terminal-container :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) { background: rgba(255,255,255,0.20); }

  .ssh-loading {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    z-index: 2;
    animation: loadFadeIn 0.3s ease;
    /* Block xterm's text cursor from bleeding through the loader area */
    background: var(--n);
    cursor: default;
  }
  .loading-text { display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .loading-title { font-size: 14px; font-weight: 500; color: var(--t2); font-family: var(--ui); }
  .loading-sub { font-size: 11px; color: var(--t4); font-family: var(--mono); }
  .loading-dots::after { content: ''; animation: dots 1.5s steps(4, end) infinite; }
  @keyframes dots { 0% { content: ''; } 25% { content: '.'; } 50% { content: '..'; } 75% { content: '...'; } }
  @keyframes loadFadeIn { from { opacity: 0; transform: scale(0.97); } to { opacity: 1; transform: scale(1); } }

  .ssh-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    background: color-mix(in srgb, var(--err) 12%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--err) 25%, var(--b1));
    flex-shrink: 0;
    z-index: 3;
  }
  .ssh-banner-text {
    font-family: var(--ui);
    font-size: 12px;
    color: var(--t2);
    flex: 1;
  }
  .ssh-banner-btn {
    padding: 4px 12px;
    border-radius: 5px;
    border: 1px solid var(--ssh);
    background: var(--ssh);
    color: #fff;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--ui);
    cursor: pointer;
  }
  .ssh-banner-btn:hover { filter: brightness(1.1); }

  .ssh-cancel-btn {
    margin-top: 14px;
    padding: 6px 16px;
    border-radius: 6px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-family: var(--ui);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    pointer-events: auto;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .ssh-cancel-btn:hover {
    background: rgba(255,255,255,0.04);
    border-color: var(--b2);
    color: var(--t1);
    cursor: pointer;
  }

  /* Find-in-terminal search bar — sits at top of active SSH tab */
  .ssh-search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    flex-shrink: 0;
    z-index: 4;
  }
  .ssh-search-icon {
    color: var(--t3);
    flex-shrink: 0;
  }
  .ssh-search-input {
    flex: 1;
    min-width: 0;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--b1);
    border-radius: var(--radius-md, 6px);
    padding: 5px 9px;
    font-size: 12.5px;
    color: var(--t1);
    font-family: var(--ui);
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .ssh-search-input::placeholder {
    color: var(--t3);
  }
  .ssh-search-input:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--acc) 12%, transparent);
  }
  .ssh-search-count {
    font-size: 11px;
    font-family: var(--mono);
    color: var(--t3);
    white-space: nowrap;
    min-width: 56px;
    text-align: right;
  }
  .ssh-search-btn {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .ssh-search-btn:hover:not(:disabled) {
    background: var(--c);
    border-color: var(--b2);
    color: var(--t1);
  }
  .ssh-search-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .ssh-search-close {
    margin-left: 2px;
  }

  .ssh-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }
  .empty-title {
    font-size: 16px;
    font-weight: 500;
    color: var(--t2);
    font-family: var(--ui);
    margin: 0;
  }
  .empty-sub {
    font-size: 13px;
    color: var(--t3);
    font-family: var(--ui);
    margin: 0;
  }
</style>
