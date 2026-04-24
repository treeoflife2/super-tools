export interface HttpResponse {
  status: number;
  status_text: string;
  headers: [string, string][];
  body: string;
  duration_ms: number;
  size_bytes: number;
}
