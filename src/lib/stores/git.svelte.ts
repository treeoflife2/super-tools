import { invoke } from "@tauri-apps/api/core";

class GitStore {
  gitBranch = $state('');
  gitFiles = $state<any[]>([]);
  gitChanges = $state<Record<string, number>>({});
  gitPanelOpen = $state(false);
  gitTab = $state('changes');
  gitCommitMsg = $state('');
  gitLoading = $state('');
  gitAhead = $state(0);
  gitBehind = $state(0);
  gitMsg = $state('');
  gitDiff = $state('');
  gitDiffFile = $state('');
  gitCommits = $state<any[]>([]);
  gitBranches = $state<any[]>([]);
  stagedFiles = $state(new Set<string>());

  showGitMsg(msg: string, duration = 3000) {
    this.gitMsg = msg;
    setTimeout(() => { if (this.gitMsg === msg) this.gitMsg = ''; }, duration);
  }

  async refreshGitStatus(projectPath: string, profileId?: string) {
    try {
      const [branch, files, aheadBehind] = await Promise.all([
        invoke<string>("get_git_branch", { projectPath }),
        invoke<any[]>("get_git_status", { projectPath }),
        invoke<number[]>("get_git_ahead_behind", { projectPath }).catch(() => [0, 0]),
      ]);
      this.gitBranch = branch;
      this.gitFiles = files;
      this.gitAhead = (aheadBehind as number[])[0] || 0;
      this.gitBehind = (aheadBehind as number[])[1] || 0;
      if (profileId) {
        this.gitChanges[profileId] = (files as any[]).length;
        this.gitChanges = { ...this.gitChanges };
      }
    } catch (_) {
      this.gitBranch = '';
      this.gitFiles = [];
      this.gitAhead = 0;
      this.gitBehind = 0;
    }
  }

  async doGitCommit(projectPath: string) {
    if (!this.gitCommitMsg.trim()) return;
    this.gitLoading = 'commit';
    try {
      const result = await invoke<string>("git_commit", { projectPath, message: this.gitCommitMsg.trim() });
      this.gitCommitMsg = '';
      this.showGitMsg(result.includes('Nothing') ? 'Nothing to commit' : 'Committed');
      await this.refreshGitStatus(projectPath);
    } catch (_) { this.showGitMsg('Commit failed'); }
    this.gitLoading = '';
  }

  async doGitCommitStaged(projectPath: string) {
    if (!this.gitCommitMsg.trim()) return;
    this.gitLoading = 'commit';
    try {
      if (this.stagedFiles.size === 0) {
        await invoke("git_commit", { projectPath, message: this.gitCommitMsg.trim() });
      } else {
        await invoke("git_commit", { projectPath, message: this.gitCommitMsg.trim() });
      }
      this.gitCommitMsg = '';
      this.stagedFiles = new Set();
      this.showGitMsg('Committed');
      await this.refreshGitStatus(projectPath);
    } catch (_) { this.showGitMsg('Commit failed'); }
    this.gitLoading = '';
  }

  async doGitPush(projectPath: string) {
    this.gitLoading = 'push';
    try {
      await invoke("git_push", { projectPath });
      this.showGitMsg('Pushed');
      await this.refreshGitStatus(projectPath);
    } catch (_) { this.showGitMsg('Push failed'); }
    this.gitLoading = '';
  }

  async doGitPull(projectPath: string) {
    this.gitLoading = 'pull';
    try {
      await invoke("git_pull", { projectPath });
      this.showGitMsg('Pulled');
      await this.refreshGitStatus(projectPath);
    } catch (_) { this.showGitMsg('Pull failed'); }
    this.gitLoading = '';
  }

  async viewDiff(projectPath: string, file: any) {
    this.gitDiffFile = file.path;
    try {
      this.gitDiff = await invoke<string>("git_diff_file", { projectPath, filePath: file.path });
    } catch (_) { this.gitDiff = 'Failed to load diff'; }
  }

  async toggleStageFile(projectPath: string, profileId: string | undefined, file: any) {
    const isStaged = this.stagedFiles.has(file.path);
    try {
      if (isStaged) {
        await invoke("git_unstage_file", { projectPath, filePath: file.path });
        this.stagedFiles.delete(file.path);
      } else {
        await invoke("git_stage_file", { projectPath, filePath: file.path });
        this.stagedFiles.add(file.path);
      }
      this.stagedFiles = new Set(this.stagedFiles);
      await this.refreshGitStatus(projectPath, profileId);
    } catch (_) {}
  }

  async loadGitHistory(projectPath: string) {
    try {
      this.gitCommits = await invoke<any[]>("git_log", { projectPath, limit: 20 });
    } catch (_) { this.gitCommits = []; }
  }

  async loadGitBranches(projectPath: string) {
    try {
      this.gitBranches = await invoke<any[]>("git_list_branches", { projectPath });
    } catch (_) { this.gitBranches = []; }
  }

  async switchBranch(projectPath: string, profileId: string | undefined, branchName: string) {
    try {
      await invoke("git_switch_branch", { projectPath, branchName });
      this.showGitMsg(`Switched to ${branchName}`);
      await this.refreshGitStatus(projectPath, profileId);
      await this.loadGitBranches(projectPath);
    } catch (e) {
      const err = String(e);
      if (err.includes('uncommitted') || err.includes('overwritten') || err.includes('changes')) {
        this.showGitMsg('Commit or stash changes first', 5000);
      } else {
        this.showGitMsg('Switch failed: ' + err.slice(0, 50), 5000);
      }
    }
  }

  async doGitStash(projectPath: string, profileId?: string) {
    try {
      await invoke("git_stash", { projectPath });
      this.showGitMsg('Stashed');
      await this.refreshGitStatus(projectPath, profileId);
    } catch (_) { this.showGitMsg('Stash failed'); }
  }

  async doGitStashPop(projectPath: string, profileId?: string) {
    try {
      await invoke("git_stash_pop", { projectPath });
      this.showGitMsg('Stash applied');
      await this.refreshGitStatus(projectPath, profileId);
    } catch (_) { this.showGitMsg('Stash pop failed'); }
  }
}

export const gitStore = new GitStore();
