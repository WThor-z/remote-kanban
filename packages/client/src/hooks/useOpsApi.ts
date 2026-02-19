import { useCallback, useMemo, useState } from 'react';
import type { OrchestratorExecutionEventsResponse } from '@opencode-vibe/protocol';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface OpsSummary {
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
}

export interface OpsExecution {
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
  items: OpsExecution[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface OpsAuditEvent {
  id: string;
  ts: string;
  orgId: string;
  actor: string;
  action: string;
  executionId?: string;
  taskId?: string;
  hostId?: string;
  traceId?: string;
  status?: string;
  details?: Record<string, unknown>;
}

export interface OpsAuditResponse {
  items: OpsAuditEvent[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface ListExecutionsQuery {
  offset?: number;
  limit?: number;
  status?: string;
  orgId?: string;
  hostId?: string;
  taskId?: string;
}

export interface ListAuditQuery {
  offset?: number;
  limit?: number;
  orgId?: string;
  action?: string;
  executionId?: string;
  taskId?: string;
}

const toQueryString = (query: Record<string, string | number | undefined>): string => {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value == null) {
      continue;
    }
    params.set(key, String(value));
  }
  const value = params.toString();
  return value ? `?${value}` : '';
};

const parseError = async (response: Response): Promise<string> => {
  try {
    const payload = (await response.json()) as { error?: string };
    if (payload.error) {
      return payload.error;
    }
  } catch {
    // ignore
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

  const fetchSummary = useCallback(async (): Promise<OpsSummary | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/v1/ops/summary`);
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      return (await response.json()) as OpsSummary;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch ops summary');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const listExecutions = useCallback(
    async (query: ListExecutionsQuery): Promise<OpsExecutionsResponse | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const queryString = toQueryString({
          offset: query.offset,
          limit: query.limit,
          status: query.status,
          orgId: query.orgId,
          hostId: query.hostId,
          taskId: query.taskId,
        });
        const response = await fetch(`${baseUrl}/api/v1/ops/executions${queryString}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as OpsExecutionsResponse;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to list executions');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const listAuditEvents = useCallback(
    async (query: ListAuditQuery): Promise<OpsAuditResponse | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const queryString = toQueryString({
          offset: query.offset,
          limit: query.limit,
          orgId: query.orgId,
          action: query.action,
          executionId: query.executionId,
          taskId: query.taskId,
        });
        const response = await fetch(`${baseUrl}/api/v1/ops/audit${queryString}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as OpsAuditResponse;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to list audit events');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const getExecutionEvents = useCallback(
    async (executionId: string, limit = 100): Promise<OrchestratorExecutionEventsResponse | null> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/v1/executions/${executionId}/events?limit=${limit}`);
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return (await response.json()) as OrchestratorExecutionEventsResponse;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load execution events');
        return null;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const stopExecution = useCallback(
    async (executionId: string): Promise<boolean> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/v1/executions/${executionId}/stop`, {
          method: 'POST',
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to stop execution');
        return false;
      } finally {
        setIsLoading(false);
      }
    },
    [baseUrl]
  );

  const sendExecutionInput = useCallback(
    async (executionId: string, content: string): Promise<boolean> => {
      setIsLoading(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/v1/executions/${executionId}/input`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content }),
        });
        if (!response.ok) {
          throw new Error(await parseError(response));
        }
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to send execution input');
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
    fetchSummary,
    listExecutions,
    listAuditEvents,
    getExecutionEvents,
    stopExecution,
    sendExecutionInput,
  };
};
