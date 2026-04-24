import { invoke } from '@tauri-apps/api/core';
import type { HttpResponse } from '$lib/types';

export async function executeRequest(requestId: string, environmentId: string): Promise<HttpResponse> {
  return invoke('execute_request', { requestId, environmentId });
}

export async function quickExecute(method: string, url: string, body: string = '', headers: [string, string][] = [], environmentId: string = '', authType: string = 'none', authData: string = '{}', bodyType: string = 'json'): Promise<HttpResponse> {
  return invoke('quick_execute', { method, url, body, headers, environmentId, authType, authData, bodyType });
}
