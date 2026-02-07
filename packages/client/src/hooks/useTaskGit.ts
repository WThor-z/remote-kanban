import { useCallback, useState } from 'react';
import { resolveApiBaseUrl } from '../config/endpoints';
import type { ConflictStrategy, GitStatus, MergeStrategy } from './useProjects';

export interface GitStatusResponse {
  taskId: string;
  projectId: string | null;
  branchName: string | null;
  actualBranch: string | null;
  mergeStrategy: MergeStrategy;
  conflictStrategy: ConflictStrategy;
  gitStatus: GitStatus;
  worktreePath: string | null;
  updatedAt: string;
}

export interface UseTaskGitResult {
  isLoading: boolean;
  error: string | null;
  clearError: () => void;
  getGitStatus: (taskId: string) => Promise<GitStatusResponse | null>;
  mergeTask: (taskId: string) => Promise<void>;
  pushTask: (taskId: string) => Promise<void>;
  cleanupTask: (taskId: string) => Promise<void>;
}

export function useTaskGit(): UseTaskGitResult {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const baseUrl = resolveApiBaseUrl();

  const clearError = useCallback(() => setError(null), []);

  const request = useCallback(async <T,>(path: string, options?: RequestInit): Promise<T | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}${path}`, options);
      if (!response.ok) {
        let message = 'Request failed';
        try {
          const errorData = await response.json();
          message = errorData.error || message;
        } catch {
          const text = await response.text();
          if (text) message = text;
        }
        throw new Error(message);
      }

      if (response.status === 204) {
        return null;
      }

      return (await response.json()) as T;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const getGitStatus = useCallback(async (taskId: string) => {
    return request<GitStatusResponse>(`/api/tasks/${taskId}/git-status`);
  }, [request]);

  const mergeTask = useCallback(async (taskId: string) => {
    await request(`/api/tasks/${taskId}/merge`, { method: 'POST' });
  }, [request]);

  const pushTask = useCallback(async (taskId: string) => {
    await request(`/api/tasks/${taskId}/push`, { method: 'POST' });
  }, [request]);

  const cleanupTask = useCallback(async (taskId: string) => {
    await request(`/api/tasks/${taskId}/cleanup`, { method: 'POST' });
  }, [request]);

  return {
    isLoading,
    error,
    clearError,
    getGitStatus,
    mergeTask,
    pushTask,
    cleanupTask,
  };
}
