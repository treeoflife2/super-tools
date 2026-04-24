import { invoke } from '@tauri-apps/api/core';
import type { Environment, EnvVariable } from '$lib/types';

export async function listEnvironments(): Promise<Environment[]> {
  return invoke('list_environments');
}

export async function createEnvironment(name: string, color: string): Promise<Environment> {
  return invoke('create_environment', { name, color });
}

export async function updateEnvironment(id: string, name: string, color: string): Promise<Environment> {
  return invoke('update_environment', { id, name, color });
}

export async function deleteEnvironment(id: string): Promise<void> {
  return invoke('delete_environment', { id });
}

export async function setDefaultEnvironment(id: string): Promise<void> {
  return invoke('set_default_environment', { id });
}

export async function listEnvVariables(environmentId: string): Promise<EnvVariable[]> {
  return invoke('list_env_variables', { environmentId });
}

export async function setEnvVariable(environmentId: string, key: string, value: string, isSecret: number): Promise<EnvVariable> {
  return invoke('set_env_variable', { environmentId, key, value, isSecret });
}

export async function updateEnvVariable(id: string, key: string, value: string, isSecret: number): Promise<EnvVariable> {
  return invoke('update_env_variable', { id, key, value, isSecret });
}

export async function deleteEnvVariable(id: string): Promise<void> {
  return invoke('delete_env_variable', { id });
}

export async function getEnvVariablesForResolution(environmentId: string): Promise<Record<string, string>> {
  return invoke('get_env_variables_for_resolution', { environmentId });
}
