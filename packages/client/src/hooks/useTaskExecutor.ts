/**
 * Hook for task execution with isolated worktrees
 *
 * This hook provides functions to start, stop, and monitor task execution
 * in isolated Git worktrees.
 */

import { useState, useCallback } from 'react';
import type { AgentType } from '@opencode-vibe/protocol';

// Types matching the Rust API
export interface ExecutionResponse {
  sessionId: string;
  taskId: string;
  status: string;
  message: string;
}

export interface SessionStatus {
  sessionId: string;
  taskId: string;
  status: string;
  state: string;
  worktreePath: string | null;
  branch: string | null;
}

export interface SessionSummary {
  sessionId: string;
  taskId: string;
  state: string;
}

export interface StartExecutionRequest {
  agentType: AgentType;
  baseBranch: string;
}

// API base URL
const getApiBaseUrl = (): string => {
  if (typeof import.meta !== 'undefined' && import.meta.env?.VITE_RUST_API_URL) {
    return import.meta.env.VITE_RUST_API_URL;
  }
  return 'http://localhost:8081';
};

export interface UseTaskExecutorResult {
  isExecuting: boolean;
  error: string | null;
  currentSession: SessionStatus | null;
  startExecution: (taskId: string, request: StartExecutionRequest) => Promise<ExecutionResponse | null>;
  stopExecution: (taskId: string) => Promise<boolean>;
  getExecutionStatus: (taskId: string) => Promise<SessionStatus | null>;
  cleanupWorktree: (taskId: string) => Promise<boolean>;
  listSessions: () => Promise<SessionSummary[]>;
  clearError: () => void;
}

export function useTaskExecutor(): UseTaskExecutorResult {
  const [isExecuting, setIsExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentSession, setCurrentSession] = useState<SessionStatus | null>(null);

  const baseUrl = getApiBaseUrl();

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  /**
   * Start task execution in an isolated worktree
   */
  const startExecution = useCallback(
    async (taskId: string, request: StartExecutionRequest): Promise<ExecutionResponse | null> => {
      setIsExecuting(true);
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/tasks/${taskId}/execute`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(request),
        });

        if (!response.ok) {
          const errorData = await response.json();
          throw new Error(errorData.error || 'Failed to start execution');
        }

        const result: ExecutionResponse = await response.json();
        return result;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
        return null;
      } finally {
        setIsExecuting(false);
      }
    },
    [baseUrl]
  );

  /**
   * Stop task execution
   */
  const stopExecution = useCallback(
    async (taskId: string): Promise<boolean> => {
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/tasks/${taskId}/stop`, {
          method: 'POST',
        });

        if (!response.ok) {
          const errorData = await response.json();
          throw new Error(errorData.error || 'Failed to stop execution');
        }

        setCurrentSession(null);
        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
        return false;
      }
    },
    [baseUrl]
  );

  /**
   * Get execution status for a task
   */
  const getExecutionStatus = useCallback(
    async (taskId: string): Promise<SessionStatus | null> => {
      try {
        const response = await fetch(`${baseUrl}/api/tasks/${taskId}/status`);

        if (response.status === 404) {
          return null;
        }

        if (!response.ok) {
          const errorData = await response.json();
          throw new Error(errorData.error || 'Failed to get execution status');
        }

        const status: SessionStatus = await response.json();
        setCurrentSession(status);
        return status;
      } catch (err) {
        // Don't set error for 404 - just means no active session
        if (err instanceof Error && !err.message.includes('404')) {
          setError(err.message);
        }
        return null;
      }
    },
    [baseUrl]
  );

  /**
   * Cleanup worktree after execution
   */
  const cleanupWorktree = useCallback(
    async (taskId: string): Promise<boolean> => {
      setError(null);
      try {
        const response = await fetch(`${baseUrl}/api/tasks/${taskId}/worktree`, {
          method: 'DELETE',
        });

        if (!response.ok && response.status !== 204) {
          const errorData = await response.json();
          throw new Error(errorData.error || 'Failed to cleanup worktree');
        }

        return true;
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
        return false;
      }
    },
    [baseUrl]
  );

  /**
   * List all active sessions
   */
  const listSessions = useCallback(async (): Promise<SessionSummary[]> => {
    try {
      const response = await fetch(`${baseUrl}/api/sessions`);

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to list sessions');
      }

      const data = await response.json();
      return data.sessions || [];
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
      return [];
    }
  }, [baseUrl]);

  return {
    isExecuting,
    error,
    currentSession,
    startExecution,
    stopExecution,
    getExecutionStatus,
    cleanupWorktree,
    listSessions,
    clearError,
  };
}
