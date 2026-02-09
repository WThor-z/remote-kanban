import { useCallback, useMemo, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export type MemoryScope = 'project' | 'host';
export type MemoryKind = 'preference' | 'constraint' | 'fact' | 'workflow';
export type MemorySource = 'auto_rule' | 'auto_llm' | 'manual';

export interface MemorySettings {
  enabled: boolean;
  gatewayStoreEnabled: boolean;
  rustStoreEnabled: boolean;
  autoWrite: boolean;
  promptInjection: boolean;
  tokenBudget: number;
  retrievalTopK: number;
  llmExtractEnabled: boolean;
}

export interface MemoryItem {
  id: string;
  hostId: string;
  projectId?: string;
  scope: MemoryScope;
  kind: MemoryKind;
  content: string;
  tags: string[];
  confidence: number;
  pinned: boolean;
  enabled: boolean;
  source: MemorySource;
  sourceTaskId?: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
  hitCount: number;
}

export interface MemoryListQuery {
  hostId?: string;
  projectId?: string;
  scope?: MemoryScope;
  kind?: MemoryKind;
  search?: string;
  enabledOnly?: boolean;
  limit?: number;
  offset?: number;
}

export interface MemoryCreateInput {
  hostId: string;
  projectId?: string;
  scope: MemoryScope;
  kind: MemoryKind;
  content: string;
  tags?: string[];
  confidence?: number;
  pinned?: boolean;
  enabled?: boolean;
  sourceTaskId?: string;
}

export interface MemoryUpdateInput {
  hostId?: string;
  content?: string;
  scope?: MemoryScope;
  kind?: MemoryKind;
  tags?: string[];
  confidence?: number;
  pinned?: boolean;
  enabled?: boolean;
}

const toQueryString = (query: Record<string, string | number | boolean | undefined>): string => {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value == null) {
      continue;
    }
    params.set(key, String(value));
  }
  const text = params.toString();
  return text ? `?${text}` : '';
};

const parseError = async (response: Response): Promise<string> => {
  try {
    const payload = (await response.json()) as { error?: string };
    if (payload.error) {
      return payload.error;
    }
  } catch {
    // ignore parse error
  }
  return `Request failed (${response.status})`;
};

export const useMemoryApi = () => {
  const baseUrl = useMemo(() => resolveApiBaseUrl(), []);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const getSettings = useCallback(
    async (hostId?: string): Promise<MemorySettings | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const query = toQueryString({ hostId });
        const response = await fetch(`${baseUrl}/api/memory/settings${query}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as MemorySettings;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch memory settings');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const patchSettings = useCallback(
    async (patch: Partial<MemorySettings>, hostId?: string): Promise<MemorySettings | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/memory/settings`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ hostId, patch }),
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as MemorySettings;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update memory settings');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const listItems = useCallback(
    async (query: MemoryListQuery): Promise<MemoryItem[]> => {
      setIsLoading(true);
      setError(null);
      try {
        const queryString = toQueryString({
          hostId: query.hostId,
          projectId: query.projectId,
          scope: query.scope,
          kind: query.kind,
          search: query.search,
          enabledOnly: query.enabledOnly,
          limit: query.limit,
          offset: query.offset,
        });
        const response = await fetch(`${baseUrl}/api/memory/items${queryString}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as MemoryItem[];
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to list memory items');
        return [];
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const createItem = useCallback(
    async (input: MemoryCreateInput): Promise<MemoryItem | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/memory/items`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(input),
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as MemoryItem;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to create memory item');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const updateItem = useCallback(
    async (id: string, patch: MemoryUpdateInput): Promise<MemoryItem | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/memory/items/${id}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(patch),
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as MemoryItem;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update memory item');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const deleteItem = useCallback(
    async (id: string, hostId?: string): Promise<boolean> => {
      setIsLoading(true);
      setError(null);
      try {
        const query = toQueryString({ hostId });
        const response = await fetch(`${baseUrl}/api/memory/items/${id}${query}`, {
          method: 'DELETE',
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to delete memory item');
        return false;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  return {
    isLoading,
    error,
    clearError,
    getSettings,
    patchSettings,
    listItems,
    createItem,
    updateItem,
    deleteItem,
  };
};
