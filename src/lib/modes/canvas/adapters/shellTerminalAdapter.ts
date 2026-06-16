import type { CanvasTabAdapter } from '$lib/modes/canvas/adapter';
import { get } from 'svelte/store';
import { shellTerminals } from '$lib/modes/canvas/stores/shellTerminalsStore';
import {
  attachShellTerminal,
  detachShellTerminal,
} from '$lib/modes/canvas/services/shellTerminalLifecycle';
import { agentKillTerminal } from '$lib/modes/agent/commands';
import { canvasDeleteTile } from '$lib/modes/canvas/commands';
import { tilesByTab } from '$lib/modes/canvas/stores/canvasStore';

export async function performShellClose(tabId: string): Promise<void> {
  const entry = get(shellTerminals).get(tabId);
  if (!entry) return;

  // Kill the PTY process.
  if (entry.terminalId) {
    await agentKillTerminal(entry.terminalId).catch(() => {});
  }

  // Dispose xterm and remove its container from the DOM.
  if (entry.internal) {
    entry.internal.term.dispose();
    entry.internal.container.parentElement?.removeChild(entry.internal.container);
  }

  // Remove from store — canvas tile disappears via the subscribe path.
  shellTerminals.update((m) => {
    const next = new Map(m);
    next.delete(tabId);
    return next;
  });

  // Clean up the persisted canvas tile row.
  const tile = get(tilesByTab).get(tabId);
  const workspaceId = tile?.workspaceId ?? '__phase2_stub__';
  await canvasDeleteTile(workspaceId, tabId).catch(() => {});
}

export const shellTerminalAdapter: CanvasTabAdapter = {
  tabKind: 'shell_terminal',
  mountStrategy: 'reparent',

  listOpenTabs(_workspaceId) {
    return [...get(shellTerminals).values()].map((e) => ({
      id: e.id,
      title: `Terminal — ${e.cwd}`,
    }));
  },

  subscribe(_workspaceId, onChange) {
    return shellTerminals.subscribe(() => onChange());
  },

  attach(id, slot) {
    attachShellTerminal(id, slot);
  },

  detach(id, slot) {
    detachShellTerminal(id, slot);
  },

  getMeta(id) {
    const e = get(shellTerminals).get(id);
    return { title: e ? `Terminal — ${e.cwd}` : id };
  },

  openInHomeMode(_id) {
    // Shell terminals are Canvas-native; no home mode equivalent.
  },

  closeTab(tabId) {
    window.dispatchEvent(
      new CustomEvent('canvas:request-shell-close', { detail: { tabId } }),
    );
  },
};
