import { invoke } from '@tauri-apps/api/core';
import type { HistoryEntry } from '$lib/types';

export async function listHistory(limit: number): Promise<HistoryEntry[]> {
  return invoke('list_history', { limit });
}

export async function clearHistory(): Promise<void> {
  return invoke('clear_history');
}

export async function deleteHistoryEntry(id: string): Promise<void> {
  return invoke('delete_history_entry', { id });
}
