import { invoke } from '@tauri-apps/api/core';

export async function githubConnect(token: string): Promise<string> {
  return invoke('github_connect', { token });
}

export async function githubDisconnect(): Promise<void> {
  return invoke('github_disconnect');
}

export async function githubGetStatus(): Promise<{ username: string; avatarUrl: string } | null> {
  return invoke('github_get_status');
}

export async function githubGetOauthUrl(): Promise<string> {
  return invoke('github_get_oauth_url');
}

export async function githubConnectWithToken(token: string): Promise<string> {
  return invoke('github_connect_with_token', { token });
}

export async function gistCheckExists(): Promise<boolean> {
  return invoke('gist_check_exists');
}

export async function gistSyncPush(): Promise<string> {
  return invoke('gist_sync_push');
}

export async function gistSyncPull(): Promise<string> {
  return invoke('gist_sync_pull');
}
