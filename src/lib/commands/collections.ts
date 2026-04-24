import { invoke } from '@tauri-apps/api/core';
import type { Collection } from '$lib/types';

export async function listCollections(): Promise<Collection[]> {
  return invoke('list_collections');
}

export async function createCollection(name: string): Promise<Collection> {
  return invoke('create_collection', { name });
}

export async function updateCollection(id: string, name: string, envId: string | null): Promise<Collection> {
  return invoke('update_collection', { id, name, envId });
}

export async function deleteCollection(id: string): Promise<void> {
  return invoke('delete_collection', { id });
}

export async function reorderCollections(ids: string[]): Promise<void> {
  return invoke('reorder_collections', { ids });
}
