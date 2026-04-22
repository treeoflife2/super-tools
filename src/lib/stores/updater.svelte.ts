class UpdaterStore {
  updateReady = $state<any>(null); // { version, body } — only set after download complete
  updateDismissed = $state(false);
  showUpdateModal = $state(false);
  showWhatsNew = $state(false);
  whatsNewBody = $state('');
  private pendingUpdate: any = null; // holds the downloaded update object

  async checkAndDownloadUpdate() {
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (!update) return;

      // Always download — Tauri updater doesn't persist downloads across restarts
      await update.download();
      this.pendingUpdate = update;
      this.updateReady = { version: update.version, body: update.body || '' };
    } catch(e) {
      // Silently ignore — no update or network issue
    }
  }

  async restartToUpdate() {
    if (!this.pendingUpdate) {
      // Re-check and download if pendingUpdate was lost
      try {
        const { check } = await import("@tauri-apps/plugin-updater");
        const update = await check();
        if (update) {
          await update.download();
          this.pendingUpdate = update;
        }
      } catch(_) {}
    }
    if (!this.pendingUpdate) return;
    try {
      await this.pendingUpdate.install();
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch(e) {
      console.error("Update restart failed:", e);
    }
  }

  checkWhatsNew(version: string) {
    const lastSeen = typeof localStorage !== 'undefined' ? localStorage.getItem('clauge-last-seen-version') : null;
    if (lastSeen && lastSeen !== version) {
      // Version changed since last launch — fetch release notes
      fetch(`https://api.github.com/repos/ansxuman/Clauge/releases/tags/v${version}`)
        .then(r => r.ok ? r.json() : null)
        .then(data => {
          if (data?.body) {
            this.whatsNewBody = data.body;
            this.showWhatsNew = true;
          }
        })
        .catch(() => {});
    }
    if (typeof localStorage !== 'undefined') localStorage.setItem('clauge-last-seen-version', version);
  }
}

export const updater = new UpdaterStore();
