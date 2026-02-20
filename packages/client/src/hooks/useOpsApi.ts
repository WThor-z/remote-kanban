import { useCallback, useMemo, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface OpsSummaryResponse {
  updatedAt: string;
  hosts: {
    total: number;
    online: number;
    busy: number;
    offline: number;
  };
  executions: {
    total: number;
    running: number;
    completed: number;
    failed: number;
    cancelled: number;
    other: number;
  };
  memory: {
    enabled: boolean;
    gatewayStoreEnabled: boolean;
    rustStoreEnabled: boolean;
    tokenBudget: number;
    retrievalTopK: number;
  };
  audit: {
    total: number;
  };
}

export interface OpsExecutionItem {
  executionId: string;
  taskId: string;
  projectId: string | null;
  workspaceId: string | null;
  hostId: string | null;
  traceId: string | null;
  orgId: string | null;
  parentExecutionId: string | null;
  agentRole: string | null;
  handoffId: string | null;
  agentType: string;
  baseBranch: string;
  status: string;
  createdAt: string;
  startedAt: string | null;
  endedAt: string | null;
  durationMs: number | null;
  summary: string | null;
  error: string | null;
  eventCount: number;
}

export interface OpsExecutionsResponse {
  items: OpsExecutionItem[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface OpsAuditEvent {
  id: string;
  timestamp: string;
  orgId: string;
  actor: string;
  action: string;
  taskId?: string;
  executionId?: string;
  hostId?: string;
  status?: string;
  detail: Record<string, unknown>;
}

export interface OpsAuditResponse {
  items: OpsAuditEvent[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface ListOpsExecutionsQuery {
  offset?: number;
  limit?: number;
  status?: string;
  taskId?: string;
  projectId?: string;
  workspaceId?: string;
  hostId?: string;
}

export interface ListOpsAuditQuery {
  offset?: number;
  limit?: number;
  action?: string;
  taskId?: string;
  executionId?: string;
  hostId?: string;
}

const toQueryString = (query: Record<string, string | number | undefined>): string => {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value == null || value === '') {
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

export const useOpsApi = () => {
  const baseUrl = useMemo(() => resolveApiBaseUrl(), []);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const getSummary = useCallback(async (): Promise<OpsSummaryResponse | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/v1/ops/summary`);
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      return (await response.json()) as OpsSummaryResponse;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch ops summary');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const listExecutions = useCallback(
    async (query: ListOpsExecutionsQuery = {}): Promise<OpsExecutionsResponse | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const queryString = toQueryString({
          offset: query.offset,
          limit: query.limit,
          status: query.status,
          taskId: query.taskId,
          projectId: query.projectId,
          workspaceId: query.workspaceId,
          hostId: query.hostId,
        });
        const response = await fetch(`${baseUrl}/api/v1/ops/executions${queryString}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as OpsExecutionsResponse;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch ops executions');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const listAudit = useCallback(
    async (query: ListOpsAuditQuery = {}): Promise<OpsAuditResponse | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const queryString = toQueryString({
          offset: query.offset,
          limit: query.limit,
          action: query.action,
          taskId: query.taskId,
          executionId: query.executionId,
          hostId: query.hostId,
        });
        const response = await fetch(`${baseUrl}/api/v1/ops/audit${queryString}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as OpsAuditResponse;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch ops audit');
        return null;
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
    getSummary,
    listExecutions,
    listAudit,
  };
};
