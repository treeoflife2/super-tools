export interface Collection {
  id: string;
  name: string;
  description: string;
  sortOrder: number;
  envId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Request {
  id: string;
  collectionId: string;
  name: string;
  description: string;
  method: string;
  url: string;
  body: string;
  bodyType: string;
  authType: string;
  authData: string;
  preScript: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface RequestHeader {
  id: string;
  requestId: string;
  key: string;
  value: string;
  enabled: number;
  sortOrder: number;
}

export interface RequestParam {
  id: string;
  requestId: string;
  key: string;
  value: string;
  enabled: number;
  sortOrder: number;
}

export interface RequestWithDetails extends Request {
  headers: RequestHeader[];
  params: RequestParam[];
}

export interface RequestUpdate {
  name?: string;
  method?: string;
  url?: string;
  body?: string;
  bodyType?: string;
  authType?: string;
  authData?: string;
  preScript?: string;
}

export interface KVInput {
  key: string;
  value: string;
  enabled: number;
}

export interface ImportResult {
  collectionsCount: number;
  requestsCount: number;
  message: string;
}
