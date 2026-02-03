import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ExecutionEvent } from '@opencode-vibe/protocol';
import { resolveApiBaseUrl } from '../config/endpoints';

export interface RunEventFilters {
  eventType?: string;
  agentEventType?: string;
}

interface RunEventsResponse {
  events: ExecutionEvent[];
  hasMore: boolean;
  nextOffset?: number;
}

export interface UseRunEventsResult {
  events: ExecutionEvent[];
  isLoading: boolean;
  error: string | null;
  hasMore: boolean;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
}

export function useRunEvents(
  taskId: string | null,
  runId: string | null,
  filters: RunEventFilters,
  limit = 200,
): UseRunEventsResult {
  const [events, setEvents] = useState<ExecutionEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [offset, setOffset] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const baseUrl = resolveApiBaseUrl();

  const filterKey = useMemo(() => `${filters.eventType || ''}|${filters.agentEventType || ''}`,[
    filters.eventType,
    filters.agentEventType,
  ]);

  const fetchPage = useCallback(async (nextOffset: number, replace: boolean) => {
    if (!taskId || !runId) {
      setEvents([]);
      setHasMore(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const params = new URLSearchParams();
      params.set('offset', String(nextOffset));
      params.set('limit', String(limit));
      if (filters.eventType) {
        params.set('eventType', filters.eventType);
      }
      if (filters.agentEventType) {
        params.set('agentEventType', filters.agentEventType);
      }

      const response = await fetch(
        `${baseUrl}/api/tasks/${taskId}/runs/${runId}/events?${params.toString()}`,
      );

      if (!response.ok) {
        if (response.status === 404) {
          setEvents([]);
          setHasMore(false);
          return;
        }
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to fetch run events');
      }

      const data: RunEventsResponse = await response.json();
      setEvents(prev => (replace ? data.events : [...prev, ...data.events]));
      setHasMore(data.hasMore);
      setOffset(data.nextOffset ?? nextOffset + data.events.length);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl, filters.agentEventType, filters.eventType, limit, runId, taskId]);

  const refresh = useCallback(async () => {
    setOffset(0);
    await fetchPage(0, true);
  }, [fetchPage]);

  const loadMore = useCallback(async () => {
    if (hasMore && !isLoading) {
      await fetchPage(offset, false);
    }
  }, [fetchPage, hasMore, isLoading, offset]);

  useEffect(() => {
    void refresh();
  }, [refresh, runId, taskId, filterKey]);

  return {
    events,
    isLoading,
    error,
    hasMore,
    refresh,
    loadMore,
  };
}
