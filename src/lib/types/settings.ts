export interface AppearanceConfig {
  theme: string;
  accentColor: string;
}

export interface HistoryEntry {
  id: string;
  requestId: string | null;
  method: string;
  url: string;
  resolvedUrl: string;
  requestBody: string;
  requestHeaders: string;
  responseStatus: number | null;
  responseBody: string | null;
  responseHeaders: string | null;
  responseSizeBytes: number | null;
  durationMs: number | null;
  environmentId: string | null;
  createdAt: string;
}
