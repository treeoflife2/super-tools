export interface Environment {
  id: string;
  name: string;
  color: string;
  isDefault: number;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface EnvVariable {
  id: string;
  environmentId: string;
  key: string;
  value: string;
  isSecret: number;
  sortOrder: number;
}
