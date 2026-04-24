import { writable } from 'svelte/store';
import type { HistoryEntry } from '$lib/types';
import * as cmd from '$lib/commands';

export const history = writable<HistoryEntry[]>([]);
export const historyOpen = writable<boolean>(false);
export const activeHistoryEntry = writable<HistoryEntry | null>(null);

export async function loadHistory(limit: number = 50) {
  try {
    const entries = await cmd.listHistory(limit);
    history.set(entries);
  } catch (err) {
    console.error('Failed to load history:', err);
  }
}

export async function clearHistory() {
  await cmd.clearHistory();
  history.set([]);
}

export async function deleteHistoryEntry(id: string) {
  await cmd.deleteHistoryEntry(id);
  history.update(h => h.filter(x => x.id !== id));
}
