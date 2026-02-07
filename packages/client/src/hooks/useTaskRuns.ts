import { useCallback, useEffect, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';

export type RunStatus =
  | 'initializing'
  | 'creating_worktree'
  | 'starting'
  | 'running'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'cleaning_up';

export interface RunSummary {
  id: string;
  taskId: string;
  agentType: string;
  promptPreview: string;
  createdAt: string;
  startedAt: string | null;
  endedAt: string | null;
  durationMs: number | null;
  status: RunStatus;
  eventCount: number;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  messageType?: string;
  toolCall?: { name: string; input: Record<string, unknown> };
  toolResult?: { success: boolean; output: string };
}

export interface UseTaskRunsResult {
  runs: RunSummary[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  loadMessages: (runId: string) => Promise<ChatMessage[]>;
}

export function useTaskRuns(taskId: string | null): UseTaskRunsResult {
  const [runs, setRuns] = useState<RunSummary[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const refresh = useCallback(async () => {
    if (!taskId) {
      setRuns([]);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(`${baseUrl}/api/tasks/${taskId}/runs`);
      if (!response.ok) {
        if (response.status === 404) {
          setRuns([]);
          return;
        }
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to fetch runs');
      }
      const data: RunSummary[] = await response.json();
      setRuns(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl, taskId]);

  const loadMessages = useCallback(async (runId: string): Promise<ChatMessage[]> => {
    if (!taskId) {
      return [];
    }

    try {
      const response = await fetch(`${baseUrl}/api/tasks/${taskId}/runs/${runId}/messages`);
      if (!response.ok) {
        if (response.status === 404) {
          return [];
        }
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to fetch messages');
      }
      const data: { messages: ChatMessage[] } = await response.json();
      return data.messages;
    } catch (err) {
      console.error('Failed to load messages:', err);
      return [];
    }
  }, [baseUrl, taskId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    runs,
    isLoading,
    error,
    refresh,
    loadMessages,
  };
}
