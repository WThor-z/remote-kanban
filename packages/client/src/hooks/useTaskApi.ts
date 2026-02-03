/**
 * Hook for interacting with the Rust Task API
 *
 * This hook provides functions to interact with the Rust backend API
 * for task CRUD operations.
 */

import { useState, useCallback } from 'react';
import type { AgentType } from '@opencode-vibe/protocol';
import { resolveApiBaseUrl } from '../config/endpoints';

// Types matching the Rust API
export type TaskStatus = 'todo' | 'in_progress' | 'in_review' | 'done';
export type TaskPriority = 'low' | 'medium' | 'high';

export interface Task {
  id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  priority: TaskPriority;
  agentType: AgentType | null;
  baseBranch: string | null;
  model: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateTaskRequest {
  title: string;
  description?: string;
  priority?: TaskPriority;
  agentType?: AgentType;
  baseBranch?: string;
  /** Target host for execution (undefined = auto select) */
  targetHost?: string;
  /** Model to use for execution (format: provider/model) */
  model?: string;
}

export interface UpdateTaskRequest {
  title?: string;
  description?: string;
  status?: TaskStatus;
  priority?: TaskPriority;
}

// API base URL - defaults to Rust backend REST API (port 8081)
const getApiBaseUrl = (): string => resolveApiBaseUrl();

export interface UseTaskApiResult {
  tasks: Task[];
  isLoading: boolean;
  error: string | null;
  fetchTasks: () => Promise<void>;
  createTask: (data: CreateTaskRequest) => Promise<Task | null>;
  getTask: (id: string) => Promise<Task | null>;
  updateTask: (id: string, data: UpdateTaskRequest) => Promise<Task | null>;
  deleteTask: (id: string) => Promise<boolean>;
  clearError: () => void;
}

export function useTaskApi(): UseTaskApiResult {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const baseUrl = getApiBaseUrl();

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const fetchTasks = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/tasks`);
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to fetch tasks');
      }
      const data: Task[] = await response.json();
      setTasks(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const createTask = useCallback(async (data: CreateTaskRequest): Promise<Task | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/tasks`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to create task');
      }
      const task: Task = await response.json();
      setTasks(prev => [task, ...prev]);
      return task;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const getTask = useCallback(async (id: string): Promise<Task | null> => {
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/tasks/${id}`);
      if (!response.ok) {
        if (response.status === 404) {
          return null;
        }
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to get task');
      }
      return await response.json();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
      return null;
    }
  }, [baseUrl]);

  const updateTask = useCallback(async (id: string, data: UpdateTaskRequest): Promise<Task | null> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/tasks/${id}`, {
        method: 'PATCH',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to update task');
      }
      const task: Task = await response.json();
      setTasks(prev => prev.map(t => t.id === id ? task : t));
      return task;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  const deleteTask = useCallback(async (id: string): Promise<boolean> => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(`${baseUrl}/api/tasks/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok && response.status !== 204) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to delete task');
      }
      setTasks(prev => prev.filter(t => t.id !== id));
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [baseUrl]);

  return {
    tasks,
    isLoading,
    error,
    fetchTasks,
    createTask,
    getTask,
    updateTask,
    deleteTask,
    clearError,
  };
}
