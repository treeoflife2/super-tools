import { invoke } from '@tauri-apps/api/core';
import type { Request, RequestWithDetails, RequestUpdate, RequestHeader, RequestParam, KVInput } from '$lib/types';

export async function listRequests(collectionId: string): Promise<Request[]> {
  return invoke('list_requests', { collectionId });
}

export async function getRequest(id: string): Promise<RequestWithDetails> {
  return invoke('get_request', { id });
}

export async function createRequest(collectionId: string, name: string, method: string): Promise<Request> {
  return invoke('create_request', { collectionId, name, method });
}

export async function updateRequest(id: string, data: RequestUpdate): Promise<Request> {
  return invoke('update_request', { id, data });
}

export async function deleteRequest(id: string): Promise<void> {
  return invoke('delete_request', { id });
}

export async function duplicateRequest(id: string): Promise<Request> {
  return invoke('duplicate_request', { id });
}

export async function moveRequest(id: string, targetCollectionId: string): Promise<Request> {
  return invoke('move_request', { id, targetCollectionId });
}

export async function updateRequestHeaders(requestId: string, headers: KVInput[]): Promise<RequestHeader[]> {
  return invoke('update_request_headers', { requestId, headers });
}

export async function updateRequestParams(requestId: string, params: KVInput[]): Promise<RequestParam[]> {
  return invoke('update_request_params', { requestId, params });
}
