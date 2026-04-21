import { invoke } from "@tauri-apps/api/core";

class ContextsStore {
  contextSnippets = $state<any[]>([]);
  contextEditing = $state<any>(null);
  contextNewName = $state('');
  contextNewContent = $state('');
  modalContexts = $state<string[]>([]);
  showContextPicker = $state<any>(false);
  modalContextEnabled = $state(false);
  showContextDropdown = $state(false);

  async loadContextSnippets() {
    try { this.contextSnippets = await invoke("get_context_snippets"); } catch(_) { this.contextSnippets = []; }
  }

  async saveContextSnippet() {
    if (!this.contextNewName.trim() || !this.contextNewContent.trim()) return;
    try {
      await invoke("save_context_snippet", { name: this.contextNewName.trim(), content: this.contextNewContent.trim() });
      this.contextNewName = ''; this.contextNewContent = '';
      this.contextEditing = null;
      await this.loadContextSnippets();
    } catch(_) {}
  }

  async deleteContextSnippet(name: string) {
    try { await invoke("delete_context_snippet", { name }); await this.loadContextSnippets(); } catch(_) {}
  }

  async attachContextsToSession(profileId: string, projectPath: string, contextNames: string[]) {
    try {
      await invoke("update_session_contexts", { id: profileId, contexts: contextNames });
      await invoke("inject_session_context", { projectPath, contextNames });
    } catch(e) { console.error('Context inject failed:', e); }
  }

  async detachContextsFromSession(profileId: string, projectPath: string) {
    try {
      await invoke("update_session_contexts", { id: profileId, contexts: [] });
      await invoke("remove_injected_context", { projectPath });
    } catch(_) {}
  }
}

export const contextsStore = new ContextsStore();
