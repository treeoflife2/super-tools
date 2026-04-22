import { invoke } from "@tauri-apps/api/core";
import { theme } from './theme.svelte';

class TerminalStore {
  terminalMap = $state(new Map());
  activeTermEntry = $state<any>(null);
  currentTerminalId = $state<string | null>(null);
  termFontSize = $state(13);
  sessionActivity = $state<Record<string, string | null>>({});
  // Per-profile context usage: profileId -> { fillPercent, totalTokens, contextWindow, model, compacted }
  contextUsage = $state<Record<string, any>>({});

  constructor() {
    if (typeof localStorage !== 'undefined') {
      this.termFontSize = parseInt(localStorage.getItem('clauge-font-size') || '13');
    }
  }

  getTermConfig() {
    return {
      theme: theme.getTermTheme(),
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", "SF Mono", "Source Code Pro", "IBM Plex Mono", "Menlo", "Monaco", "Consolas", monospace',
      fontSize: this.termFontSize,
      lineHeight: 1.4,
      cursorBlink: true,
      cursorStyle: 'bar' as const,
      scrollback: 10000,
    };
  }

  async refreshContextUsage(profileId: string, projectPath: string, sessionId: string) {
    try {
      const usage = await invoke("get_session_context_usage", {
        projectPath,
        sessionId,
      });
      this.contextUsage[profileId] = usage;
      this.contextUsage = { ...this.contextUsage };
    } catch (_) {
      // Session file might not exist yet
    }
  }

  formatContextTokens(tokens: number): string {
    if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
    if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(0)}k`;
    return String(tokens);
  }
}

export const terminalStore = new TerminalStore();
