import { writable } from 'svelte/store';
import { STORAGE_KEYS } from '$lib/shared/constants/storage';

export const githubConnected = writable<boolean>(false);
export const githubUsername = writable<string | null>(null);
export const githubAvatarUrl = writable<string | null>(null);
export const syncing = writable<boolean>(false);
// Show "Cloud data found, restore?" prompt
export const showSyncRestorePrompt = writable<boolean>(false);
// Track if first sync has been done (prevents auto-push before user decides)
export const hasSyncedOnce = writable<boolean>(
  typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEYS.HAS_SYNCED) === 'true' : false
);

export function markSynced() {
  hasSyncedOnce.set(true);
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEYS.HAS_SYNCED, 'true');
  }
}

const savedLastSynced = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEYS.LAST_SYNCED_AT) : null;
export const lastSyncedAt = writable<string | null>(savedLastSynced);

export function setConnected(username: string, avatarUrl?: string) {
  // Guard: skip if already connected with same username (prevents duplicate toasts/updates)
  const currentUsername = _lastConnectedUsername;
  if (currentUsername === username) return;
  _lastConnectedUsername = username;

  githubConnected.set(true);
  githubUsername.set(username);
  if (avatarUrl) {
    githubAvatarUrl.set(avatarUrl);
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEYS.GITHUB_AVATAR, avatarUrl);
    }
  }
}

let _lastConnectedUsername: string | null = null;

export function setDisconnected() {
  _lastConnectedUsername = null;
  githubConnected.set(false);
  githubUsername.set(null);
  githubAvatarUrl.set(null);
  hasSyncedOnce.set(false);
  if (typeof localStorage !== 'undefined') {
    localStorage.removeItem(STORAGE_KEYS.GITHUB_AVATAR);
    localStorage.removeItem(STORAGE_KEYS.HAS_SYNCED);
  }
}

// Restore avatar from localStorage on load
if (typeof localStorage !== 'undefined') {
  const savedAvatar = localStorage.getItem(STORAGE_KEYS.GITHUB_AVATAR);
  if (savedAvatar) githubAvatarUrl.set(savedAvatar);
}

export function setSyncing(isSyncing: boolean) {
  syncing.set(isSyncing);
}

export function setLastSynced(time: string) {
  lastSyncedAt.set(time);
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEYS.LAST_SYNCED_AT, time);
  }
}
