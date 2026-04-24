import { invoke } from '@tauri-apps/api/core';
import type { SqlConnectionConfig, SqlConnection, SqlQueryResult, TableInfo, ColumnInfo, SqlScript } from '$lib/types/sql';

export async function sqlConnect(config: SqlConnectionConfig): Promise<string> {
  return invoke('sql_connect', { config });
}

export async function sqlConnectDatabase(config: SqlConnectionConfig, database: string, poolKey?: string): Promise<string> {
  return invoke('sql_connect_database', { config, database, poolKey: poolKey ?? null });
}

export async function sqlDisconnect(connectionId: string): Promise<void> {
  return invoke('sql_disconnect', { connectionId });
}

export async function sqlTestConnection(config: SqlConnectionConfig): Promise<string> {
  return invoke('sql_test_connection', { config });
}

export async function sqlExecuteQuery(connectionId: string, query: string): Promise<SqlQueryResult> {
  return invoke('sql_execute_query', { connectionId, query });
}

export async function sqlListDatabases(connectionId: string): Promise<string[]> {
  return invoke('sql_list_databases', { connectionId });
}

export async function sqlCreateDatabase(connectionId: string, name: string): Promise<void> {
  return invoke('sql_create_database', { connectionId, name });
}

export async function sqlListSchemas(connectionId: string): Promise<string[]> {
  return invoke('sql_list_schemas', { connectionId });
}

export async function sqlListTables(connectionId: string, database?: string, schema?: string): Promise<TableInfo[]> {
  return invoke('sql_list_tables', { connectionId, database, schema });
}

export async function sqlDescribeTable(connectionId: string, table: string, schema?: string): Promise<ColumnInfo[]> {
  return invoke('sql_describe_table', { connectionId, table, schema });
}

export async function sqlSaveConnection(config: SqlConnectionConfig): Promise<SqlConnection> {
  return invoke('sql_save_connection', { config });
}

export async function sqlListSavedConnections(): Promise<SqlConnection[]> {
  return invoke('sql_list_saved_connections');
}

export async function sqlDeleteSavedConnection(id: string): Promise<void> {
  return invoke('sql_delete_saved_connection', { id });
}

export async function sqlUpdateSavedConnection(id: string, config: SqlConnectionConfig): Promise<SqlConnection> {
  return invoke('sql_update_saved_connection', { id, config });
}

export async function sqlSaveScript(name: string, connectionId: string | null, databaseName: string, query: string): Promise<SqlScript> {
  return invoke('sql_save_script', { name, connectionId, databaseName, query });
}

export async function sqlListScripts(): Promise<SqlScript[]> {
  return invoke('sql_list_scripts');
}

export async function sqlUpdateScript(id: string, name: string, query: string, databaseName?: string): Promise<SqlScript> {
  return invoke('sql_update_script', { id, name, query, databaseName });
}

export async function sqlDeleteScript(id: string): Promise<void> {
  return invoke('sql_delete_script', { id });
}
